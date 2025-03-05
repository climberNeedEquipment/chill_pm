use serde::{Deserialize, Serialize};
use std::fmt;
// Application state that will be shared between handlers
#[derive(Clone)]
pub struct AppState {
    pub binance_base_url: String,
    pub binance_api_key: String,
    pub binance_api_secret: String,
    pub eisen_base_url: String,
    pub reqwest_cli: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketPrices {
    pub btc: f64,
    pub eth: f64,
}

impl fmt::Display for MarketPrices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BTC: ${:.2}, ETH: ${:.2}", self.btc, self.eth)
    }
}