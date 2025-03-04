use crate::agent::othentic::OthenticAgent;
use crate::error::AppError;
use crate::executor;
use crate::executor::eisen::get_chain_metadata;
use crate::feed::binance::BinancePriceFeed;
use crate::portfolio::binance::{get_binance_portfolio, AccountSummary};
use crate::processors::{process_binance_place_order, process_eisen_swaps};
use crate::types;
use crate::utils::format;
use crate::utils::sign::BinanceKey;
use alloy::network::{EthereumWallet, TransactionBuilder, TransactionResponse};
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
pub struct ExecuteParams {
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binance_portfolio: Option<AccountSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onchain_portfolio: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<serde_json::Value>,
}

// Handler for POST /api/v1/execute
pub async fn execute(
    State(state): State<types::AppState>,
    Json(params): Json<ExecuteParams>,
) -> impl IntoResponse {
    println!(
        "Processing request with wallet address: {}",
        params.wallet_address
    );
    println!("Using Binance base URL: {}", state.binance_base_url);
    println!("Using Eisen base URL: {}", state.eisen_base_url);

    // Create a response object that we'll populate
    let mut response = ExecuteResponse {
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
    let rpc_url = reqwest::Url::parse("https://mainnet.base.org").unwrap();

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
        match executor::eisen::get_balance_allow(base_url, 8453, params.wallet_address.clone())
            .await
        {
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
        let binance_portfolio_summary =
            format::format_binance_portfolio(&binance_account_info.unwrap());

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
                                    let pretty_json =
                                        serde_json::to_string_pretty(&inner_strategy_json)
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
