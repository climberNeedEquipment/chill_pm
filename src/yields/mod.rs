mod aave;
mod eigen_layer;
mod lido;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub use aave::*;
pub use eigen_layer::*;
pub use lido::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct APR {
    pub symbol: String,
    pub deposit_apr: f64,
    pub borrow_apr: Option<f64>,
}

#[async_trait]
pub trait Yield {
    fn get_symbol() -> String;
    async fn get_apr(&self) -> Result<Vec<APR>, Box<dyn Error>>;
}
