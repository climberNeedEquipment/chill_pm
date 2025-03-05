pub mod openai;
pub mod othentic;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Define the Agent trait
#[async_trait]
pub trait Agent {
    fn set_prompt(&mut self, prompt: String) -> &mut Self;
    async fn chat(&self, messages: Vec<Message>) -> Result<String>;
    fn prompt(&self) -> &str;
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Exchanges {
    pub binance: BinanceExchange,
    pub eisen: EisenExchange,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinanceExchange {
    pub orders: Vec<BinanceOrder>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinanceOrder {
    pub position: String,
    pub token: String,
    pub amount: String,
    pub price: f64,
    pub side: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EisenExchange {
    pub swaps: Vec<EisenSwap>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EisenSwap {
    pub token_in: String,
    pub token_out: String,
    pub amount: f64,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Strategy {
    pub exchanges: Exchanges,
}
