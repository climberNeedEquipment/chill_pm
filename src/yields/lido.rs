use reqwest::Error;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct AprData {
    timeUnix: u64,
    apr: f64,
}

#[derive(Deserialize, Debug)]
struct StethAprResponse {
    data: StethAprData,
    meta: StethMeta,
}

#[derive(Deserialize, Debug)]
struct StethAprData {
    aprs: Vec<AprData>,
    smaApr: f64,
}

#[derive(Deserialize, Debug)]
struct StethMeta {
    symbol: String,
    address: String,
    chainId: u64,
}

/// Fetches the current stETH APR from Lido's API
/// Returns the SMA (Simple Moving Average) APR as a percentage
pub async fn fetch_steth_apr() -> Result<f64, Error> {
    let url = "https://eth-api.lido.fi/v1/protocol/steth/apr/sma";
    let response = reqwest::get(url).await?;

    let apr_data: StethAprResponse = response.json().await?;

    // Return the SMA APR directly from the response
    Ok(apr_data.data.smaApr)
}

/// Alternative implementation that calculates the average manually
/// from the daily APR values
pub async fn calculate_steth_apr() -> Result<f64, Error> {
    let url = "https://eth-api.lido.fi/v1/protocol/steth/apr/sma";
    let response = reqwest::get(url).await?;

    let apr_data: StethAprResponse = response.json().await?;

    // Calculate average manually from the daily values
    let total_apr: f64 = apr_data.data.aprs.iter().map(|data| data.apr).sum();
    let avg_apr = total_apr / apr_data.data.aprs.len() as f64;

    Ok(avg_apr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_steth_apr() {
        let apr = fetch_steth_apr().await.unwrap();
        println!("Current stETH APR: {:.2}%", apr);
    }

    #[tokio::test]
    async fn test_calculate_steth_apr() {
        let apr = calculate_steth_apr().await.unwrap();
        println!("Current stETH APR: {:.2}%", apr);
    }
}
