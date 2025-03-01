use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PriceData {
    pub timestamp: u128,
    pub market_price: Option<f64>,
    pub buy_long_price: Option<f64>,
    pub sell_short_price: Option<f64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarketIndexResponse {
    mark_price: String,             // mark price
    index_price: String,            // index price
    estimated_settle_price: String, // Estimated Settle Price, only useful in the last hour before the settlement starts.
    last_funding_rate: String,      // This is the Latest funding rate
    next_funding_time: u64,
    interest_rate: String,
    time: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepthResponse {
    last_update_id: u64,
    bids: Vec<(String, String)>,
    asks: Vec<(String, String)>,
}

pub async fn fetch_binance_prices(client: &ReqwestClient, symbol: &String) -> Result<PriceData> {
    let mut price_data = PriceData {
        timestamp: 0,
        market_price: None,
        buy_long_price: None,
        sell_short_price: None,
    };

    // Fetch the market index price
    let market_response: MarketIndexResponse = client
        .get("https://fapi.binance.com/fapi/v1/premiumIndex")
        .query(&[("symbol", symbol)]) // Fix the query formatting
        .send()
        .await?
        .json()
        .await?;

    price_data.market_price = Some(market_response.mark_price.parse::<f64>().unwrap());
    price_data.timestamp = market_response.time.into();

    // Fetch the order book depth
    let response: DepthResponse = client
        .get("https://fapi.binance.com/fapi/v1/depth")
        .query(&[("symbol", symbol.as_str()), ("limit", "5")]) // Correct the format here
        .send()
        .await?
        .json()
        .await?;

    price_data.buy_long_price = Some(response.asks[0].0.parse::<f64>().unwrap());
    price_data.sell_short_price = Some(response.bids[0].0.parse::<f64>().unwrap());

    Ok(price_data)
}

/// Fetches prices for both BTC and ETH in parallel
pub async fn fetch_major_crypto_prices(client: &ReqwestClient) -> Result<HashMap<String, PriceData>> {
    let btc_symbol = "BTCUSDT".to_string();
    let eth_symbol = "ETHUSDT".to_string();
    
    // Fetch both prices in parallel
    let (btc_result, eth_result) = tokio::join!(
        fetch_binance_prices(client, &btc_symbol),
        fetch_binance_prices(client, &eth_symbol)
    );
    
    // Create a HashMap to store the results
    let mut prices = HashMap::new();
    
    // Add BTC price data if successful
    match btc_result {
        Ok(price_data) => {
            prices.insert("BTC".to_string(), price_data);
        },
        Err(err) => {
            println!("Error fetching BTC price: {:?}", err);
        }
    }
    
    // Add ETH price data if successful
    match eth_result {
        Ok(price_data) => {
            prices.insert("ETH".to_string(), price_data);
        },
        Err(err) => {
            println!("Error fetching ETH price: {:?}", err);
        }
    }
    
    Ok(prices)
}
