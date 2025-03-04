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
    pub cur_funding_rate: Option<f64>,
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
        cur_funding_rate: None,
    };
    // Fetch the market index price
    let market_response: MarketIndexResponse = client
        .get("https://testnet.binancefuture.com/fapi/v1/premiumIndex")
        .query(&[("symbol", symbol)]) // Fix the query formatting
        .send()
        .await?
        .json()
        .await?;

    price_data.market_price = Some(market_response.mark_price.parse::<f64>().unwrap());
    price_data.timestamp = market_response.time.into();

    // Fetch the order book depth
    let response: DepthResponse = client
        .get("https://fapi.binance.comfapi/v1/depth")
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
pub async fn fetch_major_crypto_prices(
    client: &ReqwestClient,
) -> Result<HashMap<String, PriceData>> {
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
        }
        Err(err) => {
            println!("Error fetching BTC price: {:?}", err);
        }
    }

    // Add ETH price data if successful
    match eth_result {
        Ok(price_data) => {
            prices.insert("ETH".to_string(), price_data);
        }
        Err(err) => {
            println!("Error fetching ETH price: {:?}", err);
        }
    }

    Ok(prices)
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[tokio::test]
    async fn test_fetch_binance_prices() {
        // Skip this test if running in CI environment
        if std::env::var("CI").is_ok() {
            println!("Skipping Binance API test in CI environment");
            return;
        }

        // Create a real client
        let client = reqwest::Client::new();
        
        // Test with a real symbol
        let symbol = "BTCUSDT".to_string();
        
        // Test the function with the actual Binance API
        let result = fetch_binance_prices(&client, &symbol).await;
        
        // Check the result
        assert!(result.is_ok(), "Failed to fetch prices: {:?}", result.err());
        let price_data = result.unwrap();
        
        // Verify we got reasonable data
        assert!(price_data.timestamp > 0, "Expected non-zero timestamp");
        assert!(price_data.market_price.is_some(), "Expected market price to be present");
        assert!(price_data.buy_long_price.is_some(), "Expected buy price to be present");
        assert!(price_data.sell_short_price.is_some(), "Expected sell price to be present");
        
        // Print the results for debugging
        println!("BTC Price Data: {:?}", price_data);
        
        if let Some(market_price) = price_data.market_price {
            assert!(market_price > 0.0, "Expected positive market price");
        }
    }

    #[tokio::test]
    async fn test_fetch_major_crypto_prices() {
        // Skip this test if running in CI environment
        if std::env::var("CI").is_ok() {
            println!("Skipping Binance API test in CI environment");
            return;
        }

        // Create a real client
        let client = reqwest::Client::new();
        
        // Test the function with the actual Binance API
        let result = fetch_major_crypto_prices(&client).await;
        
        // Check the result
        assert!(result.is_ok(), "Failed to fetch major crypto prices: {:?}", result.err());
        let prices = result.unwrap();
        
        // Verify we got data for both BTC and ETH
        assert!(prices.contains_key("BTC"), "Expected BTC price data");
        assert!(prices.contains_key("ETH"), "Expected ETH price data");
        
        // Print the results for debugging
        println!("Major Crypto Prices: {:?}", prices);
        
        // Check BTC data
        if let Some(btc_data) = prices.get("BTC") {
            assert!(btc_data.market_price.is_some(), "Expected BTC market price to be present");
            if let Some(market_price) = btc_data.market_price {
                assert!(market_price > 0.0, "Expected positive BTC market price");
            }
        }
        
        // Check ETH data
        if let Some(eth_data) = prices.get("ETH") {
            assert!(eth_data.market_price.is_some(), "Expected ETH market price to be present");
            if let Some(market_price) = eth_data.market_price {
                assert!(market_price > 0.0, "Expected positive ETH market price");
            }
        }
    }
}
