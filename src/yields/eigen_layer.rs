// // https://api.eigenexplorer.com/stakers/{address}

// // etherfi
// // https://api.sevenseas.capital/info
// // ['ethereum', 'arbitrum', 'avalanche', 'base', 'corn', 'bnb', 'sonic', 'swell', 'bob'
// // https://api.sevenseas.capital/etherfi/apy/<network>/<vault_address>
// //

// // symbiotic etherfi
// // https://api.sevenseas.capital/etherfi/ethereum/performance/0x917ceE801a67f933F2e6b33fC0cD1ED2d5909D88
// // karak etherfi
// // https://api.sevenseas.capital/etherfi/ethereum/performance/0x7223442cad8e9cA474fC40109ab981608F8c4273

// // https://app.ether.fi/api/portfolio/v3/{address}
// use reqwest::Error;
// use serde::Deserialize;

// struct Protocols {
//     data: HashMap<String, f64>,
// }

// #[derive(Deserialize, Debug)]
// struct VaultData {
//     address: String,
//     block: u64,
//     timestamp: String,
//     apy: Protocols,
//     #[serde(rename = "7_day_apy")]
//     seven_day_apy: Protocols,
//     #[serde(rename = "14_day_apy")]
//     fourteen_day_apy: Protocols,
//     #[serde(rename = "30_day_apy")]
//     thirty_day_apy: Protocols,
//     allocation: Protocols,
// }

// #[derive(Deserialize, Debug)]
// struct ApiResponse {
//     response: Vec<VaultData>,
// }

// // kelp dao
// // https://universe.kelpdao.xyz/rseth/totalApy
// // https://universe.kelpdao.xyz/rseth/gainApy
use super::{Yield, APR};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct EigenYield {
    pub date: DateTime<Utc>,
    pub eth_staking_apr: f64,
    pub eigen_staking_apr: f64,
    pub total_eth_tvl: f64,
    pub total_eigen_tvl: f64,
    pub eth_price_usd: f64,
    pub eigen_price_usd: f64,
    pub eth_eigen_price_ratio: f64,
}

// Define structures to parse the Dune API response
#[derive(Debug, Deserialize)]
struct DuneResponse {
    result: DuneResult,
}

#[derive(Debug, Deserialize)]
struct DuneResult {
    rows: Vec<DuneRow>,
}

#[derive(Debug, Deserialize)]
struct DuneRow {
    total_eth_tvl: f64,
    total_eigen_tvl: f64,
    eigen_price_usd: f64,
    eth_price_usd: f64,
    eth_staking_apr: f64,
    eigen_staking_apr: f64,
    eth_eigen_price_ratio: f64,
}
async fn fetch_eigen_apr() -> Result<EigenYield, Box<dyn Error>> {
    // You'll need to get an API key from Dune Analytics
    let dune_api_key = std::env::var("DUNE_API_KEY").expect("DUNE_API_KEY must be set");
    let client = reqwest::Client::new();

    // Query ID for the APR calculation
    let query_id = "4127474"; // Updated to your actual query ID
    let response = client
        .get(format!(
            "https://api.dune.com/api/v1/query/{}/results?limit=1000",
            query_id
        ))
        .header("X-Dune-API-Key", &dune_api_key)
        .send()
        .await?;

    let data: DuneResponse = response.json().await?;

    // Extract the first row which contains the APR data
    let row = data.result.rows.first().ok_or("No data found")?;

    Ok(EigenYield {
        date: Utc::now(), // Current time as we don't have a specific date in the response
        eth_staking_apr: row.eth_staking_apr,
        eigen_staking_apr: row.eigen_staking_apr,
        total_eth_tvl: row.total_eth_tvl,
        total_eigen_tvl: row.total_eigen_tvl,
        eth_price_usd: row.eth_price_usd,
        eigen_price_usd: row.eigen_price_usd,
        eth_eigen_price_ratio: row.eth_eigen_price_ratio,
    })
}

#[derive(Debug, Deserialize)]
pub struct Eigen {}
#[async_trait]
impl Yield for Eigen {
    fn get_symbol() -> String {
        "eigenlayer".to_string()
    }

    async fn get_apr(&self) -> Result<Vec<APR>, Box<dyn Error>> {
        let apr = fetch_eigen_apr().await?;
        Ok(vec![
            APR {
                symbol: "StrategyBase(EIGEN)".to_string(),
                deposit_apr: apr.eigen_staking_apr,
                borrow_apr: None,
            },
            APR {
                symbol: "StrategyBase(ETH)".to_string(),
                deposit_apr: apr.eth_staking_apr,
                borrow_apr: None,
            },
        ])
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_eigen_apr() {
        dotenv::dotenv().ok();
        let apr = fetch_eigen_apr().await.unwrap();
        println!("Eigen Layer APR: {:.2}%", apr.eigen_staking_apr);
    }
}
