use crate::agent::othentic::OthenticAgent;
use crate::agent::Strategy;
use crate::error::AppError;
use crate::executor;
use crate::executor::eisen::fetch_chain_portfolio;
use crate::executor::eisen::ChainPortfolio;
use crate::feed::binance::BinancePriceFeed;
use crate::portfolio::binance::fetch_binance_portfolio;
use crate::portfolio::binance::AccountInfo;
use crate::processors::{process_binance_place_order, process_eisen_swaps};
use crate::types;
use crate::types::MarketPrices;
use crate::utils::format;
use crate::utils::sign::BinanceKey;
use crate::yields::Yield;
use crate::yields::CombinedYields;
use crate::yields::CombinedYieldFetcher;
use crate::yields::APR;
use crate::yields::{Aave, Eigen, Lido};
use alloy::network::EthereumWallet;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use reqwest;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error as StdError;
use std::io::{self, Error as IoError};

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
pub struct ExecuteStrategyParams {
    pub wallet_address: String,
    pub model: Option<String>,
}

fn format_json(value: &serde_json::Value) -> Result<String, AppError> {
    serde_json::to_string_pretty(value)
        .map_err(|e| AppError::internal_error(format!("Failed to format JSON: {}", e)))
}

async fn fetch_prices(
    binance_base_url: &String,
    reqwest_cli: &reqwest::Client,
) -> Result<types::MarketPrices, AppError> {
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

    Ok(types::MarketPrices {
        eth: eth_price.mark_price.parse::<f64>().unwrap_or(0.0),
        btc: btc_price.mark_price.parse::<f64>().unwrap_or(0.0),
    })
}

fn get_provider(rpc_url: &String) -> Result<Box<dyn Provider>, Box<dyn StdError>> {
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

    Ok(Box::new(provider))
}

async fn fetch_chain_data(
    eisen_base_url: &String,
    rpc_url: &String,
) -> Result<executor::eisen::ChainData, Box<dyn StdError>> {
    // Get provider
    let provider = get_provider(rpc_url)?;

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

    let chain_data = match executor::eisen::get_chain_metadata(eisen_base_url, chain_id).await {
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
#[serde(rename_all = "camelCase")]
pub struct ExecuteStrategyResponse {
    pub status: String,
    pub message: String,
    pub binance_portfolio: AccountInfo,
    pub onchain_portfolio: ChainPortfolio,
    pub strategy: Strategy,
}

// Handler for POST /api/v1/execute
pub async fn execute_strategy(
    State(state): State<types::AppState>,
    Json(params): Json<ExecuteStrategyParams>,
) -> Result<impl IntoResponse, AppError> {
    println!(
        "Processing request with wallet address: {}",
        params.wallet_address
    );
    println!("Using Binance base URL: {}", state.binance_base_url);
    println!("Using Eisen base URL: {}", state.eisen_base_url);
    let base_rpc_url: String = "https://mainnet.base.org".to_string();
    // Create a Binance key from the API credentials
    let binance_key: BinanceKey = BinanceKey {
        api_key: state.binance_api_key.clone(),
        secret_key: state.binance_api_secret.clone(),
    };
    let provider =
        get_provider(&base_rpc_url).map_err(|e| AppError::internal_error(e.to_string()))?;
    println!("Fetching crypto prices from Binance...");
    let market_prices: MarketPrices =
        fetch_prices(&state.binance_base_url, &state.reqwest_cli).await?;
    let price_data = format!("Market price:\n{}", market_prices);

    println!("Price data: {}", price_data);
    println!("Fetching Binance portfolio data...");

    let binance_portfolio = fetch_binance_portfolio(&state.binance_base_url, &binance_key)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    println!("Binance portfolio: {:?}", binance_portfolio);
    println!("Wallet address: {}", params.wallet_address);

    let chain_data = fetch_chain_data(&state.eisen_base_url, &base_rpc_url)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;
    let onchain_portfolio =
        executor::eisen::fetch_chain_portfolio(&state.eisen_base_url, 8453, &params.wallet_address)
            .await
            .map_err(|e| AppError::internal_error(e.to_string()))?;
    println!("Base chain portfolio: {:#?}", onchain_portfolio);

    let portfolio_str = format!(
        "{}\n\n{:#?}",
        format::format_binance_portfolio(&binance_portfolio),
        onchain_portfolio
    );

    println!("Fetching yields...");
    let yield_fetcher = CombinedYieldFetcher::new();
    let yields = yield_fetcher.get_apr().await.map_err(|e| AppError::internal_error(e.to_string()))?;
    let yield_str = format!("Yields: {}", yields);
    println!("Yields: {}", yield_str);

    let othentic_agent = OthenticAgent::new("localhost".to_string(), 4003, Some("0".to_string()));
    println!(
        "Fetching strategy from Othentic... \n\n{}\n\n{}\n\n{}",
        price_data, portfolio_str, yield_str
    );
    let strategy = othentic_agent
        .get_strategy(
            &params.model.unwrap_or("o1".to_string()),
            &price_data,
            &portfolio_str,
            &yield_str,
        )
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    println!("{:#?}", strategy);
    process_binance_place_order(&strategy, &state.binance_base_url, &binance_key)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    // Convert wallet address string to alloy Address type

    process_eisen_swaps(
        &strategy,
        &provider,
        &state.eisen_base_url,
        &chain_data,
        &params.wallet_address,
    )
    .await
    .map_err(|e| AppError::internal_error(e.to_string()))?;

    println!("Strategy executed");
    // Create a response object that we'll populate
    let response = ExecuteStrategyResponse {
        status: "success".to_string(),
        message: "Strategy executed".to_string(),
        binance_portfolio,
        onchain_portfolio,
        strategy,
    };

    Ok((StatusCode::OK, Json(response)))
}

#[derive(Debug, Deserialize)]
pub struct GetPortfolioParams {
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPortfolioResponse {
    pub status: String,
    pub message: String,
    pub binance_portfolio: AccountInfo,
    pub onchain_portfolio: ChainPortfolio,
    pub prices: types::MarketPrices,
}

pub async fn get_portfolio(
    State(state): State<types::AppState>,
    Query(params): Query<GetPortfolioParams>,
) -> Result<impl IntoResponse, AppError> {
    println!(
        "Processing portfolio request with wallet address: {}",
        params.wallet_address
    );

    let prices: MarketPrices = fetch_prices(&state.binance_base_url, &state.reqwest_cli).await?;

    let binance_key: BinanceKey = BinanceKey {
        api_key: state.binance_api_key.clone(),
        secret_key: state.binance_api_secret.clone(),
    };
    let onchain_portfolio =
        fetch_chain_portfolio(&state.eisen_base_url, 8453, &params.wallet_address)
            .await
            .map_err(|e| AppError::internal_error(e.to_string()))?;

    let binance_portfolio = fetch_binance_portfolio(&state.binance_base_url, &binance_key)
        .await
        .map_err(|e| AppError::internal_error(e.to_string()))?;

    let response = GetPortfolioResponse {
        status: "success".to_string(),
        message: "Portfolio fetched".to_string(),
        binance_portfolio,
        onchain_portfolio,
        prices,
    };

    Ok((StatusCode::OK, Json(response)))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetYieldsResponse {
    pub status: String,
    pub message: String,
    pub yields: CombinedYields
}

pub async fn get_yields() -> Result<impl IntoResponse, AppError> {

    let combined_yield_fetcher = CombinedYieldFetcher::new();

    let yields = combined_yield_fetcher.get_apr().await.map_err(|e| AppError::internal_error(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(GetYieldsResponse {
            status: "success".to_string(),
            message: "Yields fetched".to_string(),
            yields,
        }),
    ))
}
