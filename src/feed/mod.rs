use async_trait::async_trait;
use std::error::Error;

pub mod binance;
pub mod service;

#[async_trait]
pub trait Feed<T> {
    async fn feed(&self) -> Result<T, Box<dyn Error + Send + Sync>>;
}

#[async_trait]
pub trait Processor<T, G>
where
    T: Into<G> + Send + Sync,
{
    async fn process(&self, data: &G) -> Result<T, Box<dyn Error + Send + Sync>>;
}
