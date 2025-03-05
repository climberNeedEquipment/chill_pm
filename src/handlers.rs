use crate::agent::othentic::OthenticAgent;
use crate::agent::Strategy;
use crate::error::AppError;
use crate::executor;
use crate::executor::eisen::get_chain_metadata;
use crate::feed::binance::BinancePriceFeed;
use crate::portfolio::binance::AccountInfo;
use crate::portfolio::binance::{get_binance_portfolio, AccountSummary};
use crate::processors::{process_binance_place_order, process_eisen_swaps};
use crate::types;
use crate::utils::format;
use crate::utils::sign::BinanceKey;
use crate::executor::eisen::ChainPortfolio;
use alloy::network::EthereumWallet;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use reqwest;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error as StdError;
use std::io::{self, Error as IoError};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub message: String,
}

// Handler for GET /health
pub async fn health_check() -> Result<impl IntoResponse, AppError> {
    let response = HealthCheckResponse {
        status: "ok".to_string(),
        message: "Server is running".to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}

#[derive(Debug, Deserialize)]
pub struct GenerateStrategyParams {
    pub wallet_address: String,
}

fn format_json(value: &serde_json::Value) -> Result<String, AppError> {
    serde_json::to_string_pretty(value)
        .map_err(|e| AppError::internal_error(format!("Failed to format JSON: {}", e)))
}

async fn fetch_prices(
    binance_base_url: &String,
    reqwest_cli: &reqwest::Client,
) -> Result<serde_json::Value, AppError> {
    let btcsymbol = "BTCUSDT".to_string();
    let btc_price_feed = BinancePriceFeed::new(binance_base_url, reqwest_cli, &btcsymbol);

    let ethsymbol = "ETHUSDT".to_string();
    let eth_price_feed = BinancePriceFeed::new(binance_base_url, reqwest_cli, &ethsymbol);

    let btc_price = btc_price_feed
        .fetch_index_price()
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;
    let eth_price = eth_price_feed
        .fetch_index_price()
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    let mut price_data = serde_json::Map::new();

    price_data.insert(
        "BTC".to_string(),
        serde_json::Value::String(
            btc_price
                .mark_price
                .parse::<f64>()
                .unwrap_or(0.0)
                .to_string(),
        ),
    );

    price_data.insert(
        "ETH".to_string(),
        serde_json::Value::String(
            eth_price
                .mark_price
                .parse::<f64>()
                .unwrap_or(0.0)
                .to_string(),
        ),
    );

    Ok(serde_json::Value::Object(price_data))
}

async fn fetch_chain_data(
    eisen_base_url: &String,
    rpc_url: &String,
) -> Result<executor::eisen::ChainData, Box<dyn StdError>> {
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
            return Err(Box::new(IoError::new(
                io::ErrorKind::NotFound,
                "PRIVATE_KEY_DEPLOYER not set in environment",
            )));
        }
    };

    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .wallet(wallet.clone())
        .on_http(reqwest::Url::parse(rpc_url).unwrap());
    let provider = Arc::new(provider);

    // Get chain metadata
    let chain_id = match provider.get_chain_id().await {
        Ok(id) => id,
        Err(err) => {
            println!("Error getting chain ID: {:?}", err);
            return Err(Box::new(IoError::new(
                io::ErrorKind::Other,
                err.to_string(),
            )));
        }
    };

    let chain_data = match get_chain_metadata(eisen_base_url, chain_id).await {
        Ok(data) => data,
        Err(err) => {
            println!("Error getting chain metadata: {:?}", err);
            return Err(Box::new(IoError::new(
                io::ErrorKind::Other,
                err.to_string(),
            )));
        }
    };

    Ok(chain_data)
}

#[derive(Debug, Serialize)]
pub struct GenerateStrategyResponse {
    pub status: String,
    pub message: String,
    pub binance_portfolio: AccountInfo,
    pub onchain_portfolio: ChainPortfolio,
    pub strategy: Strategy,
}

// Handler for POST /api/v1/execute
pub async fn generate_strategy(
    State(state): State<types::AppState>,
    Json(params): Json<GenerateStrategyParams>,
) -> Result<impl IntoResponse, AppError> {
    println!(
        "Processing request with wallet address: {}",
        params.wallet_address
    );
    println!("Using Binance base URL: {}", state.binance_base_url);
    println!("Using Eisen base URL: {}", state.eisen_base_url);

    // Create a Binance key from the API credentials
    let binance_key: BinanceKey = BinanceKey {
        api_key: state.binance_api_key.clone(),
        secret_key: state.binance_api_secret.clone(),
    };

    println!("Fetching crypto prices from Binance...");
    let price_data =
        format_json(&fetch_prices(&state.binance_base_url, &state.reqwest_cli).await?)?;
    println!("Price data: {}", price_data);
    println!("Fetching Binance portfolio data...");

    let binance_account_info = get_binance_portfolio(&state.binance_base_url, &binance_key)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    println!("Wallet address: {}", params.wallet_address);

    let base_rpc_url = "https://mainnet.base.org".to_string();
    let chain_data = fetch_chain_data(&state.eisen_base_url, &base_rpc_url)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;
    let base_chain_portfolio =
        executor::eisen::fetch_chain_portfolio(&state.eisen_base_url, 8543, &params.wallet_address)
            .await
            .map_err(|e| AppError::internal_error(e.to_string()))?;
    println!("Base chain portfolio: {:?}", base_chain_portfolio);

    let binance_portfolio = format!(
        "{}\n\n{:#?}",
        format::format_binance_portfolio(&binance_account_info),
        base_chain_portfolio
    );
    let model = "o1".to_string();
    let othentic_agent = OthenticAgent::new("localhost".to_string(), 4003, Some("0".to_string()));
    let strategy = othentic_agent
        .get_strategy(&model, &price_data, &binance_portfolio)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    println!("Strategy: {:?}", strategy);
    process_binance_place_order(&strategy, &state.binance_base_url, &binance_key)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    let rpc_url = base_rpc_url
        .parse()
        .map_err(|e| AppError::internal_error(format!("Invalid RPC URL: {}", e)))?;
    // Create a provider for the Base network
    let provider = ProviderBuilder::new().on_http(rpc_url);
    // Convert wallet address string to alloy Address type
    let wallet_addr = params
        .wallet_address
        .parse::<alloy::primitives::Address>()
        .map_err(|e| AppError::internal_error(format!("Invalid wallet address: {}", e)))?;

    process_eisen_swaps(
        &strategy,
        &provider,
        &state.eisen_base_url,
        &chain_data,
        &wallet_addr,
    )
    .await
    .map_err(|e| AppError::internal_error(e.to_string()))?;

    // Create a response object that we'll populate
    let response = GenerateStrategyResponse {
        status: "success".to_string(),
        message: "Strategy executed".to_string(),
        binance_portfolio: binance_account_info,
        onchain_portfolio: base_chain_portfolio,
        strategy: strategy,
    };

    Ok((StatusCode::OK, Json(response)))
}
