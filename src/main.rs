use anyhow::Result;
use axum::{
    http::{header, Method},
    routing::{get, post},
    Router,
};
use clap::Parser;
use dotenv::dotenv;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
pub mod agent;
pub mod cli;
pub mod constants;
pub mod error;
pub mod executor;
pub mod feed;
pub mod handlers;
pub mod portfolio;
pub mod processors;
pub mod types;
pub mod utils;
pub mod yields;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv()?;
    let args: cli::Args = cli::Args::parse();
    //  Lets use testnet for now
    let binance_base_url = "https://testnet_binancefuture.com".to_string();

    // Get API credentials from environment variables
    let binance_api_key =
        env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY must be set in environment variables");
    let binance_api_secret = env::var("BINANCE_API_SECRET")
        .expect("BINANCE_API_SECRET must be set in environment variables");
    let eisen_base_url =
        env::var("EISEN_BASE_URL").expect("EISEN_BASE_URL must be set in environment variables");

    // Create shared state
    let state = types::AppState {
        binance_base_url,
        binance_api_key,
        binance_api_secret,
        eisen_base_url,
        reqwest_cli: reqwest::Client::new(),
    };

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/v1/execute", post(handlers::execute_strategy))
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
                ])
                // Allow credentials (cookies, etc.)
                .allow_credentials(true),
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

// Function to call quote_and_send_tx for Eisen swaps
