use crate::utils::sign::BinanceKey;
use anyhow::Result;
use dotenv::dotenv;
use http::Request;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Asset {
    asset: String,
    wallet_balance: String,
    unrealized_profit: String,
    margin_balance: String,
    maint_margin: String,
    initial_margin: String,
    available_balance: String,
    update_time: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Position {
    symbol: String,
    position_side: String,
    position_amt: String,
    unrealized_profit: String,
    notional: String,
    initial_margin: String,
    maint_margin: String,
    update_time: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    total_maint_margin: String,
    total_wallet_balance: String,
    total_unrealized_profit: String,
    total_margin_balance: String,
    total_position_initial_margin: String,
    total_open_order_initial_margin: String,
    available_balance: String,
    assets: Vec<Asset>,
    positions: Vec<Position>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    total_initial_margin: String,
    total_maint_margin: String,
    total_wallet_balance: String,
    total_unrealized_profit: String,
    total_margin_balance: String,
}

pub async fn get_binance_portfolio(base_url: &str, key: &BinanceKey) -> Result<AccountInfo> {
    // Create an empty parameter map to sign
    let params: HashMap<String, String> = HashMap::new();

    // Sign the parameters
    let signed_params = key
        .sign(params)
        .map_err(|e| anyhow::anyhow!("Error signing parameters: {}", e))?;

    // Construct the full URL with the signed query string
    let url = format!(
        "{}/fapi/v3/account?{}",
        base_url,
        serde_urlencoded::to_string(signed_params)?
    );

    // Create a client and set the necessary headers
    let client = Client::new();
    let response = client
        .get(&url)
        .header(
            "X-MBX-APIKEY",
            HeaderValue::from_str(&key.api_key)
                .map_err(|e| anyhow::anyhow!("Invalid API key: {}", e))?,
        )
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch account info: HTTP {}",
            response.status()
        ));
    }

    let account_info: AccountInfo = response.json().await?;
    Ok(account_info)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_binance_portfolio() -> Result<()> {
    dotenv().unwrap();
    let binance_key = BinanceKey {
        api_key: env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY must be set in .env"),
        secret_key: env::var("BINANCE_API_SECRET").expect("BINANCE_SECRET_KEY must be set in .env"),
    };
    let binance_base_url =
        if env::var("ENVIRONMENT").expect("BINANCE_ENV must be set in .env") == "test" {
            "https://testnet.binancefuture.com"
        } else {
            "https://fapi.binance.com"
        };
    let portfolio = get_binance_portfolio(&binance_base_url, &binance_key).await?;

    println!("{:?}", portfolio);
    Ok(())
}
