use crate::agent::othentic::OthenticAgent;
use crate::feed::binance::BinancePriceFeed;
use crate::portfolio::binance::{get_binance_portfolio, AccountSummary};
use crate::utils::sign::BinanceKey;
use crate::utils::format;
use alloy::network::{EthereumWallet, TransactionBuilder, TransactionResponse};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::{header, Method, StatusCode},
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
use tower_http::cors::{Any, CorsLayer};
pub mod agent;
pub mod cli;
pub mod constants;
pub mod executor;
pub mod feed;
pub mod portfolio;
pub mod utils;
pub mod yields;

use crate::utils::parser::{extract_binance_place_order, extract_eisen_swaps};

#[derive(Debug, Deserialize)]
struct ApiParams {
    wallet_address: String,
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
        .with_state(state)
        .layer(
            // Configure CORS middleware
            CorsLayer::new()
                // Allow requests from any origin
                .allow_origin(Any)
                // Allow common HTTP methods
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                // Specify explicit headers instead of Any when credentials are true
                .allow_headers([
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::ACCEPT,
                    header::ORIGIN,
                    header::HeaderName::from_static("x-requested-with"),
                    header::HeaderName::from_static("access-control-request-method"),
                    header::HeaderName::from_static("access-control-request-headers"),
                ]),
        );

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

    let btcsymbol = "BTCUSDT".to_string();
    let btc_price_feed =
        BinancePriceFeed::new(&state.binance_base_url, &state.reqwest_cli, &btcsymbol);

    let ethsymbol = "ETHUSDT".to_string();
    let eth_price_feed =
        BinancePriceFeed::new(&state.binance_base_url, &state.reqwest_cli, &ethsymbol);

    let btc_price = btc_price_feed.fetch_index_price().await;
    let eth_price = eth_price_feed.fetch_index_price().await;

    // Create a JSON object containing BTC and ETH prices
    let mut price_data = serde_json::Map::new();

    if let Ok(btc) = &btc_price {
        price_data.insert(
            "BTC".to_string(),
            serde_json::Value::String(btc.mark_price.parse::<f64>().unwrap_or(0.0).to_string()),
        );
        println!(
            "BTC price: {}",
            btc.mark_price.parse::<f64>().unwrap_or(0.0)
        );
    } else {
        println!("Failed to fetch BTC price");
    }

    if let Ok(eth) = &eth_price {
        price_data.insert(
            "ETH".to_string(),
            serde_json::Value::String(eth.mark_price.parse::<f64>().unwrap_or(0.0).to_string()),
        );
        println!(
            "ETH price: {}",
            eth.mark_price.parse::<f64>().unwrap_or(0.0)
        );
    } else {
        println!("Failed to fetch ETH price");
    }

    let price_json = serde_json::Value::Object(price_data);
    println!("Combined price data: {}", price_json);

    // Convert the price_json to a string for easier handling
    let price_json_string = price_json.to_string();
    println!("Stringified price data: {}", price_json_string);

    println!("Fetching Binance portfolio data...");

    // Get Binance portfolio data
    let binance_account_info =
        match get_binance_portfolio(&state.binance_base_url, &binance_key).await {
            Ok(account_info) => {
                println!("Successfully retrieved Binance portfolio");

                // Format and print the portfolio data
                println!("{}", format::format_binance_portfolio(&account_info));

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

    // Setup provider for Eisen swaps
    let base_url = &state.eisen_base_url;
    let rpc_url = Url::parse("https://mainnet.base.org").unwrap();

    // Get private key from environment
    let signer: PrivateKeySigner = match env::var("PRIVATE_KEY_DEPLOYER") {
        Ok(key) => {
            key.chars()
                .skip(2) // Skip "0x" prefix
                .collect::<String>()
                .parse()
                .unwrap_or_else(|_| {
                    println!("Error parsing private key");
                    panic!("Invalid private key format");
                })
        }
        Err(_) => {
            println!("PRIVATE_KEY_DEPLOYER not set in environment");
            response.status = "error".to_string();
            response.message = "PRIVATE_KEY_DEPLOYER not set in environment".to_string();
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let wallet_addr = signer.address();
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .wallet(wallet.clone())
        .on_http(rpc_url);
    let provider = Arc::new(provider);

    // Get chain metadata
    let chain_id = match provider.get_chain_id().await {
        Ok(id) => id,
        Err(err) => {
            println!("Error getting chain ID: {:?}", err);
            response.status = "error".to_string();
            response.message = format!("Error getting chain ID: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let chain_data = match get_chain_metadata(base_url, chain_id).await {
        Ok(data) => data,
        Err(err) => {
            println!("Error getting chain metadata: {:?}", err);
            response.status = "error".to_string();
            response.message = format!("Error getting chain metadata: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let base_chain_portfolio = 
        match executor::eisen::get_balance_allow(base_url, 8453, params.wallet_address.clone()).await {
            Ok(portfolio) => {
                println!("Successfully retrieved base chain portfolio");
                println!("Portfolio: {:?}", portfolio);
                Some(portfolio)
            }
            Err(err) => {
                println!("Error retrieving base chain portfolio: {:?}", err);
                response.message = format!("Error retrieving base chain portfolio: {}", err);
                None
            }
        };

    // If we have at least one portfolio, consider it a success
    // Otherwise return a 404 Not Found status
    if binance_account_info.is_some() || base_chain_portfolio.is_some() {
        response.status = "success".to_string();
        println!("Generating investment strategy...");

        // Create the specialized yield farming agent
        let yield_agent = OthenticAgent::new("localhost".to_string(), 4003, Some("0".to_string()));

        // Use the token value we already extracted above
        let portfolio_summary = format!("{:?}", base_chain_portfolio.unwrap());
        let binance_portfolio_summary = format::format_binance_portfolio(&binance_account_info.unwrap());

        let all_portfolio_summary =
            format!("{}\n\n{}", portfolio_summary, binance_portfolio_summary).to_string();

        let model = "o1".to_string();
        // Try to generate a farming strategy
        match yield_agent
            .get_strategy(&model, &price_json_string, &all_portfolio_summary)
            .await
        {
            Ok(strategy_text) => {
                println!("Successfully generated strategy");
                // Try to parse the strategy as JSON
                match serde_json::from_str::<serde_json::Value>(&strategy_text) {
                    Ok(strategy_json) => {
                        if let Some(strategy_str) = strategy_json
                            .get("data")
                            .and_then(|d| d.get("strategy"))
                            .and_then(|s| s.as_str())
                        {
                            // Parse the inner strategy string
                            match serde_json::from_str::<serde_json::Value>(strategy_str) {
                                Ok(inner_strategy_json) => {
                                    response.strategy = Some(strategy_json.clone());

                                    // Pretty print the strategy JSON for better readability
                                    let pretty_json = serde_json::to_string_pretty(&inner_strategy_json)
                                        .unwrap_or_else(|_| strategy_text.clone());
                                    println!("Strategy (pretty printed):\n{}", pretty_json);

                                    // Execute the strategy
                                    println!("Executing strategy...");
                                    // Process Binance orders
                                    // No need to get binance_orders and eisen_swaps here
                                    match process_binance_place_order(
                                        &inner_strategy_json,
                                        &state.binance_base_url,
                                        &binance_key,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            println!("Successfully executed Binance orders");
                                        }
                                        Err(err) => {
                                            println!("Error executing Binance orders: {:?}", err);
                                            // Don't fail the whole request if Binance orders fail
                                        }
                                    }

                                    // Process Eisen swaps
                                    match process_eisen_swaps(
                                        &inner_strategy_json,
                                        &provider,
                                        base_url,
                                        &chain_data,
                                        &wallet_addr,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            println!("Successfully executed Eisen swaps");
                                        }
                                        Err(err) => {
                                            println!("Error executing Eisen swaps: {:?}", err);
                                            // Don't fail the whole request if Eisen swaps fail
                                        }
                                    }
                                }
                                Err(err) => {
                                    println!(
                                        "Warning: Strategy response is not valid JSON: {}",
                                        err
                                    );
                                    // Still include the text response as a JSON string
                                    response.strategy =
                                        Some(serde_json::Value::String(strategy_text));
                                }
                            }
                        }
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

    // Print the swaps that will be executed
    println!("Swaps to be executed:");
    for (i, swap) in swaps.iter().enumerate() {
        println!("Swap {}: {} -> {} (amount: {})", 
            i + 1, 
            swap.token_in, 
            swap.token_out, 
            swap.amount);
    }
    
    if swaps.is_empty() {
        println!("No swaps to execute");
    }

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

    // Print the positions that will be executed
    println!("Positions to be executed:");
    for (i, position) in positions.iter().enumerate() {
        println!("Position {}: {:?}", i + 1, position);
    }
    
    if positions.is_empty() {
        println!("No positions to execute");
    }

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
