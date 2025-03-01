use crate::agent::openai::{OpenAIAgent, StableYieldFarmingAgent};
use crate::portfolio::binance::{get_binance_portfolio, AccountInfo, AccountSummary};
use crate::portfolio::eisen::get_onchain_portfolio;
use crate::utils::price::{fetch_binance_prices, fetch_major_crypto_prices};
use crate::utils::sign::BinanceKey;
use alloy::network::{EthereumWallet, TransactionBuilder, TransactionResponse};
use alloy::primitives::U256;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use clap::Parser;
use dotenv::dotenv;
use executor::eisen::get_chain_metadata;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use std::{env, sync::Arc};
pub mod agent;
pub mod cli;
pub mod constants;
pub mod executor;
pub mod feed;
pub mod portfolio;
pub mod utils;

use crate::utils::parser::{extract_binance_place_order, extract_eisen_swaps};

#[derive(Debug, Deserialize)]
struct ApiParams {
    wallet_address: String,
    token: Option<String>, // Optional token for Eisen portfolio
}

#[derive(Debug, Serialize)]
struct ApiResponse {
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    binance_portfolio: Option<AccountSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    onchain_portfolio: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strategy: Option<serde_json::Value>,
}

// Application state that will be shared between handlers
#[derive(Clone)]
struct AppState {
    binance_base_url: String,
    binance_api_key: String,
    binance_api_secret: String,
    eisen_base_url: String,
    reqwest_cli: reqwest::Client,
    openai_api_key: String,
}

// Helper function to format Binance portfolio data
fn format_binance_portfolio(account_info: &AccountInfo) -> String {
    let mut output = String::new();

    output.push_str(&format!("Binance Portfolio Summary:\n"));
    output.push_str(&format!(
        "Wallet Balance: {}\n",
        account_info.total_wallet_balance
    ));
    output.push_str(&format!(
        "Unrealized Profit: {}\n",
        account_info.total_unrealized_profit
    ));
    output.push_str(&format!(
        "Margin Balance: {}\n",
        account_info.total_margin_balance
    ));

    // Format assets
    if !account_info.assets.is_empty() {
        output.push_str("\nAssets:\n");
        for (i, asset) in account_info.assets.iter().enumerate().take(5) {
            output.push_str(&format!(
                "  Asset {}: {} - Balance: {}\n",
                i + 1,
                asset.asset,
                asset.wallet_balance
            ));
        }
        if account_info.assets.len() > 5 {
            output.push_str(&format!(
                "  ... and {} more assets\n",
                account_info.assets.len() - 5
            ));
        }
    }

    // Format positions
    let active_positions: Vec<_> = account_info
        .positions
        .iter()
        .filter(|p| p.position_amt != "0")
        .collect();

    if !active_positions.is_empty() {
        output.push_str("\nActive Positions:\n");
        for (i, position) in active_positions.iter().enumerate().take(5) {
            output.push_str(&format!(
                "  Position {}: {} - Amount: {}, Unrealized PnL: {}\n",
                i + 1,
                position.symbol,
                position.position_amt,
                position.unrealized_profit
            ));
        }
        if active_positions.len() > 5 {
            output.push_str(&format!(
                "  ... and {} more positions\n",
                active_positions.len() - 5
            ));
        }
    }

    output
}

// Helper function to format Eisen onchain data
fn format_onchain_data<T>(onchain_data: &T) -> String
where
    T: std::fmt::Debug,
{
    format!("Onchain Portfolio Data:\n{:#?}", onchain_data)
}

// Helper function to format token exposure data
fn format_token_exposure(
    token_exposure: &portfolio::eisen::UserOnchainPortfolio,
    token: &str,
) -> String {
    let mut output = String::new();

    output.push_str(&format!("Token Exposure for {}:\n", token));
    output.push_str(&format!(
        "Total Exposure: {}\n",
        token_exposure.total_exposure
    ));

    // Format chain details
    if !token_exposure.chain_details.is_empty() {
        output.push_str("\nChain Details:\n");
        for chain in token_exposure.chain_details.iter().take(3) {
            output.push_str(&format!("  Chain ID: {}\n", chain.chain_id));

            if !chain.protocol_details.is_empty() {
                for protocol in chain.protocol_details.iter().take(3) {
                    output.push_str(&format!("    Protocol: {}\n", protocol.name));

                    if !protocol.assets.is_empty() {
                        for asset in protocol.assets.iter().take(3) {
                            output.push_str(&format!(
                                "      Asset: {}, Balance: {}, Underlying: {}\n",
                                asset.symbol, asset.balance, asset.underlying_amount
                            ));
                        }
                        if protocol.assets.len() > 3 {
                            output.push_str(&format!(
                                "      ... and {} more assets\n",
                                protocol.assets.len() - 3
                            ));
                        }
                    }
                }
                if chain.protocol_details.len() > 3 {
                    output.push_str(&format!(
                        "    ... and {} more protocols\n",
                        chain.protocol_details.len() - 3
                    ));
                }
            }
        }
        if token_exposure.chain_details.len() > 3 {
            output.push_str(&format!(
                "  ... and {} more chains\n",
                token_exposure.chain_details.len() - 3
            ));
        }
    }

    output
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv()?;
    let args: cli::Args = cli::Args::parse();
    let signer: PrivateKeySigner = env::var("PRIVATE_KEY_DEPLOYER")
        .expect("PRIVATE_KEY must be set in .env")
        .chars()
        .skip(2)
        .collect::<String>()
        .parse()
        .unwrap();

    let wallet_addr = signer.address();
    let wallet = EthereumWallet::from(signer);

    // Set Binance base URL based on environment
    let binance_base_url = if args.environment == "test" {
        "https://testnet.binancefuture.com".to_string()
    } else {
        "https://fapi.binance.com".to_string()
    };

    let base_url = env::var("EISEN_BASE_URL").expect("EISEN_BASE_URL must be set in .env");
    let rpc_url = Url::parse("https://mainnet.base.org").unwrap();
    let provider = ProviderBuilder::new()
        .wallet(wallet.clone())
        .on_http(rpc_url);
    let provider = Arc::new(provider);

    let chain_id = provider.get_chain_id().await?;

    let chain_data = get_chain_metadata(&base_url, chain_id).await?;

    // Get API credentials from environment variables
    let binance_api_key =
        env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY must be set in environment variables");
    let binance_api_secret = env::var("BINANCE_API_SECRET")
        .expect("BINANCE_API_SECRET must be set in environment variables");
    let eisen_base_url =
        env::var("EISEN_BASE_URL").expect("EISEN_BASE_URL must be set in environment variables");
    let openai_api_key =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in environment variables");

    let reqwest_cli = reqwest::Client::new();

    // Create shared state
    let state = AppState {
        binance_base_url,
        binance_api_key,
        binance_api_secret,
        eisen_base_url,
        reqwest_cli,
        openai_api_key,
    };

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/execute", post(execute))
        .with_state(state);

    // Run the server with CLI-configured host and port
    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .expect("Failed to parse host:port into a socket address");

    println!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Handler for GET /health
async fn health_check() -> impl IntoResponse {
    let response = ApiResponse {
        status: "ok".to_string(),
        message: "Server is running".to_string(),
        binance_portfolio: None,
        onchain_portfolio: None,
        strategy: None,
    };

    (StatusCode::OK, Json(response))
}

// Handler for POST /api/v1/execute
async fn execute(
    State(state): State<AppState>,
    Json(params): Json<ApiParams>,
) -> impl IntoResponse {
    println!(
        "Processing request with wallet address: {}",
        params.wallet_address
    );
    println!("Using Binance base URL: {}", state.binance_base_url);
    println!("Using Eisen base URL: {}", state.eisen_base_url);

    // Create a response object that we'll populate
    let mut response = ApiResponse {
        status: "success".to_string(),
        message: "Portfolio data retrieved".to_string(),
        binance_portfolio: None,
        onchain_portfolio: None,
        strategy: None,
    };

    // Create a Binance key from the API credentials
    let binance_key = BinanceKey {
        api_key: state.binance_api_key.clone(),
        secret_key: state.binance_api_secret.clone(),
    };

    println!("Fetching major crypto prices from Binance...");

    // Fetch BTC and ETH prices
    let prices = match fetch_major_crypto_prices(&state.reqwest_cli).await {
        Ok(prices) => {
            println!("Successfully fetched major crypto prices:");

            if let Some(btc_price) = prices.get("BTC") {
                println!("BTC Market Price: {:?}", btc_price.market_price);
                println!("BTC Buy Price: {:?}", btc_price.buy_long_price);
                println!("BTC Sell Price: {:?}", btc_price.sell_short_price);
            }

            if let Some(eth_price) = prices.get("ETH") {
                println!("ETH Market Price: {:?}", eth_price.market_price);
                println!("ETH Buy Price: {:?}", eth_price.buy_long_price);
                println!("ETH Sell Price: {:?}", eth_price.sell_short_price);
            }
            prices
        }
        Err(err) => {
            println!("Error fetching major crypto prices: {:?}", err);
            response.status = "error".to_string();
            response.message = "Failed to retrieve any portfolio data".to_string();
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // Create a stringified version of the prices data
    let prices_str = match serde_json::to_string_pretty(&prices) {
        Ok(json_str) => {
            println!("Prices JSON: {}", json_str);
            json_str
        }
        Err(err) => {
            println!("Error serializing prices to JSON: {:?}", err);
            "{}".to_string() // Return empty JSON object on error
        }
    };

    println!("Fetching Binance portfolio data...");

    // Get Binance portfolio data
    let binance_account_info =
        match get_binance_portfolio(&state.binance_base_url, &binance_key).await {
            Ok(account_info) => {
                println!("Successfully retrieved Binance portfolio");

                // Format and print the portfolio data
                println!("{}", format_binance_portfolio(&account_info));

                // Create a simplified account summary
                let summary = AccountSummary {
                    total_initial_margin: account_info.total_position_initial_margin.clone(),
                    total_maint_margin: account_info.total_maint_margin.clone(),
                    total_wallet_balance: account_info.total_wallet_balance.clone(),
                    total_unrealized_profit: account_info.total_unrealized_profit.clone(),
                    total_margin_balance: account_info.total_margin_balance.clone(),
                };

                response.binance_portfolio = Some(summary);
                Some(account_info)
            }
            Err(err) => {
                println!("Error retrieving Binance portfolio: {:?}", err);
                response.message = format!("Error retrieving Binance portfolio: {}", err);
                None
            }
        };
    println!("Wallet address: {}", params.wallet_address);

    let onchain_portfolio =
        match get_onchain_portfolio(&state.eisen_base_url, &params.wallet_address).await {
            Ok(onchain_data) => {
                println!("Successfully retrieved raw onchain data");
                println!("{}", format_onchain_data(&onchain_data));
                Some(onchain_data)
            }
            Err(err) => {
                println!("Error retrieving onchain portfolio: {:?}", err);
                response.message = format!("Error retrieving onchain portfolio: {}", err);
                None
            }
        };

    // If we have at least one portfolio, consider it a success
    // Otherwise return a 404 Not Found status
    if binance_account_info.is_some() || onchain_portfolio.is_some() {
        response.status = "success".to_string();
        println!("Generating investment strategy...");

        // Create OpenAI agent
        let openai_agent = OpenAIAgent::new(state.openai_api_key.clone(), "o1".to_string(), 0.7);

        // Create the specialized yield farming agent
        let yield_agent = StableYieldFarmingAgent::new(openai_agent);

        // Use the token value we already extracted above
        let portfolio_summary = format_onchain_data(&onchain_portfolio.unwrap());
        let binance_portfolio_summary = format_binance_portfolio(&binance_account_info.unwrap());

        let all_portfolio_summary =
            format!("{}\n\n{}", portfolio_summary, binance_portfolio_summary).to_string();

        // Try to generate a farming strategy
        match yield_agent
            .get_farming_strategy(&prices_str, &portfolio_summary)
            .await
        {
            Ok(strategy_text) => {
                println!("Successfully generated strategy");

                // Try to parse the strategy as JSON
                match serde_json::from_str::<serde_json::Value>(&strategy_text) {
                    Ok(strategy_json) => {
                        response.strategy = Some(strategy_json);
                    }
                    Err(err) => {
                        println!("Warning: Strategy response is not valid JSON: {}", err);
                        // Still include the text response as a JSON string
                        response.strategy = Some(serde_json::Value::String(strategy_text));
                    }
                }
            }
            Err(err) => {
                println!("Error generating strategy: {:?}", err);
                // Don't fail the whole request if strategy generation fails
                // Just log the error and continue
            }
        }

        println!("Returning response with status: {}", response.status);
        (StatusCode::OK, Json(response)).into_response()
    } else {
        response.status = "error".to_string();
        response.message = "Failed to retrieve any portfolio data".to_string();
        println!("Returning 404 error: {}", response.message);
        (StatusCode::NOT_FOUND, Json(response)).into_response()
    }
}

// Error handling
enum AppError {
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let response = ApiResponse {
            status: "error".to_string(),
            message,
            binance_portfolio: None,
            onchain_portfolio: None,
            strategy: None,
        };

        (status, Json(response)).into_response()
    }
}

// Function to call quote_and_send_tx for Eisen swaps
async fn process_eisen_swaps(
    json_response: &serde_json::Value,
    provider: &dyn Provider,
    base_url: &str,
    chain_data: &executor::eisen::ChainData,
    wallet_addr: &alloy::primitives::Address,
) -> Result<(), Box<dyn Error>> {
    let swaps = extract_eisen_swaps(json_response);

    for swap in swaps {
        // Call the quote_and_send_tx function from executor/eisen
        let result = executor::eisen::quote_and_send_tx(
            provider,
            base_url,
            chain_data,
            &swap.token_in,
            &swap.token_out,
            swap.amount,
            wallet_addr,
            100, // Default slippage of 1% (100 basis points)
        )
        .await?;

        // Handle the result as needed
        println!("Eisen swap executed: {:?}", result);
    }

    Ok(())
}

// Function to process Binance positions from the strategy JSON
async fn process_binance_place_order(
    json_response: &serde_json::Value,
    binance_base_url: &str,
    binance_key: &utils::sign::BinanceKey,
) -> Result<(), Box<dyn Error>> {
    let positions = extract_binance_place_order(json_response);

    for position in positions {
        // Call the place_binance_order function
        let result = executor::binance::place_binance_order(
            binance_base_url,
            binance_key,
            &position.symbol, // Use token directly as symbol is constructed inside the function
            position.side,
            position.quantity,
            position.price,
            None, // No stop price for now
        )
        .await?;

        // Handle the result as needed
        println!("Binance position executed: {:?}", result);
    }

    Ok(())
}
