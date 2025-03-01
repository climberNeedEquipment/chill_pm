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
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::portfolio::binance::{get_binance_portfolio, AccountInfo, AccountSummary};
use crate::portfolio::eisen::{get_onchain_portfolio, get_token_exposure_onchain};
use crate::utils::sign::BinanceKey;

pub mod constants;
pub mod executor;
pub mod feed;
pub mod portfolio;
pub mod utils;
pub mod cli;

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
}

// Application state that will be shared between handlers
#[derive(Clone)]
struct AppState {
    binance_base_url: String,
    binance_api_key: String,
    binance_api_secret: String,
    eisen_base_url: String,
    reqwest_cli: reqwest::Client,
}

// Helper function to format Binance portfolio data
fn format_binance_portfolio(account_info: &AccountInfo) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("Binance Portfolio Summary:\n"));
    output.push_str(&format!("Wallet Balance: {}\n", account_info.total_wallet_balance));
    output.push_str(&format!("Unrealized Profit: {}\n", account_info.total_unrealized_profit));
    output.push_str(&format!("Margin Balance: {}\n", account_info.total_margin_balance));
    
    // Format assets
    if !account_info.assets.is_empty() {
        output.push_str("\nAssets:\n");
        for (i, asset) in account_info.assets.iter().enumerate().take(5) {
            output.push_str(&format!("  Asset {}: {} - Balance: {}\n", 
                i+1, asset.asset, asset.wallet_balance));
        }
        if account_info.assets.len() > 5 {
            output.push_str(&format!("  ... and {} more assets\n", account_info.assets.len() - 5));
        }
    }
    
    // Format positions
    let active_positions: Vec<_> = account_info.positions.iter()
        .filter(|p| p.position_amt != "0")
        .collect();
    
    if !active_positions.is_empty() {
        output.push_str("\nActive Positions:\n");
        for (i, position) in active_positions.iter().enumerate().take(5) {
            output.push_str(&format!("  Position {}: {} - Amount: {}, Unrealized PnL: {}\n", 
                i+1, position.symbol, position.position_amt, position.unrealized_profit));
        }
        if active_positions.len() > 5 {
            output.push_str(&format!("  ... and {} more positions\n", active_positions.len() - 5));
        }
    }
    
    output
}

// Helper function to format Eisen onchain data
fn format_onchain_data<T>(onchain_data: &T) -> String 
where 
    T: std::fmt::Debug
{
    format!("Onchain Portfolio Data:\n{:#?}", onchain_data)
}

// Helper function to format token exposure data
fn format_token_exposure(
    token_exposure: &portfolio::eisen::UserOnchainPortfolio, 
    token: &str
) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("Token Exposure for {}:\n", token));
    output.push_str(&format!("Total Exposure: {}\n", token_exposure.total_exposure));
    
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
                            output.push_str(&format!("      Asset: {}, Balance: {}, Underlying: {}\n", 
                                asset.symbol, asset.balance, asset.underlying_amount));
                        }
                        if protocol.assets.len() > 3 {
                            output.push_str(&format!("      ... and {} more assets\n", 
                                protocol.assets.len() - 3));
                        }
                    }
                }
                if chain.protocol_details.len() > 3 {
                    output.push_str(&format!("    ... and {} more protocols\n", 
                        chain.protocol_details.len() - 3));
                }
            }
        }
        if token_exposure.chain_details.len() > 3 {
            output.push_str(&format!("  ... and {} more chains\n", 
                token_exposure.chain_details.len() - 3));
        }
    }
    
    output
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv()?;
    let args: cli::Args = cli::Args::parse();
    
    // Set Binance base URL based on environment
    let binance_base_url = if args.environment == "test" {
        "https://testnet.binancefuture.com".to_string()
    } else {
        "https://fapi.binance.com".to_string()
    };

    // Get API credentials from environment variables
    let binance_api_key = env::var("BINANCE_API_KEY")
        .expect("BINANCE_API_KEY must be set in environment variables");
    let binance_api_secret = env::var("BINANCE_API_SECRET")
        .expect("BINANCE_API_SECRET must be set in environment variables");
    let eisen_base_url = env::var("EISEN_BASE_URL")
        .expect("EISEN_BASE_URL must be set in environment variables");

    let reqwest_cli = reqwest::Client::new();

    // Create shared state
    let state = AppState {
        binance_base_url,
        binance_api_key,
        binance_api_secret,
        eisen_base_url,
        reqwest_cli,
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
    };
    
    (StatusCode::OK, Json(response))
}

// Handler for POST /api/v1/execute
async fn execute(
    State(state): State<AppState>,
    Json(params): Json<ApiParams>,
) -> impl IntoResponse {
    println!("Processing request with wallet address: {}", params.wallet_address);
    println!("Using Binance base URL: {}", state.binance_base_url);
    println!("Using Eisen base URL: {}", state.eisen_base_url);
    
    // Create a response object that we'll populate
    let mut response = ApiResponse {
        status: "success".to_string(),
        message: "Portfolio data retrieved".to_string(),
        binance_portfolio: None,
        onchain_portfolio: None,
    };
    
    // Create a Binance key from the API credentials
    let binance_key = BinanceKey {
        api_key: state.binance_api_key.clone(),
        secret_key: state.binance_api_secret.clone(),
    };
    
    println!("Fetching Binance portfolio data...");
    
    // Track if we have any successful fetches
    let mut binance_success = false;
    let mut eisen_success = false;
    
    // Get Binance portfolio data
    match get_binance_portfolio(&state.binance_base_url, &binance_key).await {
        Ok(account_info) => {
            println!("Successfully retrieved Binance portfolio");
            
            // Format and print the portfolio data
            let portfolio_str = format_binance_portfolio(&account_info);
            println!("{}", portfolio_str);
            
            // Create a simplified account summary
            let summary = AccountSummary {
                total_initial_margin: account_info.total_position_initial_margin.clone(),
                total_maint_margin: account_info.total_maint_margin.clone(),
                total_wallet_balance: account_info.total_wallet_balance.clone(),
                total_unrealized_profit: account_info.total_unrealized_profit.clone(),
                total_margin_balance: account_info.total_margin_balance.clone(),
            };
            
            response.binance_portfolio = Some(summary);
            binance_success = true;
        },
        Err(err) => {
            println!("Error retrieving Binance portfolio: {:?}", err);
            response.message = format!("Error retrieving Binance portfolio: {}", err);
        }
    }
    
    // Get Eisen onchain portfolio data
    let token = params.token.unwrap_or_else(|| "eth".to_string());
    println!("Fetching Eisen onchain portfolio data for token: {}", token);
    println!("Wallet address: {}", params.wallet_address);
    
    match get_onchain_portfolio(&state.eisen_base_url, &params.wallet_address).await {
        Ok(onchain_data) => {
            println!("Successfully retrieved raw onchain data");
            
            // Format and print the onchain data
            let onchain_str = format_onchain_data(&onchain_data);
            println!("{}", onchain_str);
            
            match get_token_exposure_onchain(onchain_data, &token).await {
                Ok(token_exposure) => {
                    println!("Successfully retrieved token exposure for: {}", token);
                    
                    // Format and print the token exposure data
                    let exposure_str = format_token_exposure(&token_exposure, &token);
                    println!("{}", exposure_str);
                    
                    // Convert to JSON for response
                    let json_value = serde_json::to_value(&token_exposure).unwrap_or_default();
                    response.onchain_portfolio = Some(json_value);
                    eisen_success = true;
                },
                Err(err) => {
                    println!("Error retrieving token exposure: {:?}", err);
                    response.message = format!("Error retrieving token exposure: {}", err);
                }
            }
        },
        Err(err) => {
            println!("Error retrieving onchain portfolio: {:?}", err);
            response.message = format!("Error retrieving onchain portfolio: {}", err);
        }
    }
    
    // If we have at least one portfolio, consider it a success
    // Otherwise return a 404 Not Found status
    if binance_success || eisen_success {
        response.status = "success".to_string();
        println!("Returning response with status: {}", response.status);
        (StatusCode::OK, Json(response))
    } else {
        response.status = "error".to_string();
        response.message = "Failed to retrieve any portfolio data".to_string();
        println!("Returning 404 error: {}", response.message);
        (StatusCode::NOT_FOUND, Json(response))
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
        };
        
        (status, Json(response)).into_response()
    }
}
