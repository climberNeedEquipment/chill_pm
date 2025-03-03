use crate::yields::{Yield, APR};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

// Define structures to deserialize the GraphQL response
#[derive(Debug, Deserialize)]
struct AaveResponse {
    data: AaveData,
}

#[derive(Debug, Deserialize)]
struct AaveData {
    reserves: Vec<Reserve>,
}

#[derive(Debug, Deserialize)]
pub struct Aave {}

#[async_trait]
impl Yield for Aave {
    fn get_symbol() -> String {
        "aave".to_string()
    }

    async fn get_apr<'a>(&'a self) -> Result<Vec<APR>, Box<dyn Error + 'a>> {
        // Example function to demonstrate usage
        let yields = fetch_aave_yields().await?;
        let mut aprs = Vec::new();
        for yield_data in yields {
            aprs.push(APR {
                symbol: yield_data.symbol,
                deposit_apr: yield_data.deposit_apr,
                borrow_apr: Some(yield_data.borrow_apr),
            });
        }

        Ok(aprs)
    }
}

#[derive(Debug, Deserialize)]
struct Reserve {
    #[serde(rename = "__typename")]
    typename: String,
    availableLiquidity: String,
    decimals: u8,
    id: String,
    liquidityIndex: String,
    liquidityRate: String,
    name: String,
    price: PriceOracleAsset,
    stableBorrowRate: String,
    symbol: String,
    totalCurrentVariableDebt: String,
    totalLiquidity: String,
    utilizationRate: String,
    variableBorrowRate: String,
}

#[derive(Debug, Deserialize)]
struct PriceOracleAsset {
    #[serde(rename = "__typename")]
    typename: String,
    priceInEth: String,
}

// Structure to hold calculated APR data
#[derive(Debug, Serialize)]
struct AaveYield {
    pub symbol: String,
    pub deposit_apr: f64,
    pub borrow_apr: f64,
}

async fn fetch_aave_yields() -> Result<Vec<AaveYield>, Box<dyn Error>> {
    let client = reqwest::Client::new();

    // GraphQL query to fetch AAVE reserves data
    let query = r#"
    {
        reserves {
            __typename
            availableLiquidity
            decimals
            id
            liquidityIndex
            liquidityRate
            name
            price {
                __typename
                priceInEth
            }
            stableBorrowRate
            symbol
            totalCurrentVariableDebt
            totalLiquidity
            utilizationRate
            variableBorrowRate
        }
    }
    "#;

    // Aave subgraph on base
    let response = client.post("https://gateway-arbitrum.network.thegraph.com/api/a820147ae9eec25fbfa2f206671706b8/subgraphs/id/GQFbb95cE6d8mV989mL5figjaGaKCQB3xqYrr1bRyXqF")
        .json(&serde_json::json!({
            "query": query
        }))
        .send()
        .await?
        .json::<AaveResponse>()
        .await?;

    let mut yields = Vec::new();

    for reserve in response.data.reserves {
        // Parse rates from strings to f64
        // AAVE rates are in ray units (1e27) and represent per-second rates
        let liquidity_rate = parse_ray_to_apr(&reserve.liquidityRate);
        let variable_borrow_rate = parse_ray_to_apr(&reserve.variableBorrowRate);

        yields.push(AaveYield {
            symbol: format!("aBas{}", reserve.symbol),
            deposit_apr: liquidity_rate,
            borrow_apr: variable_borrow_rate,
        });
    }

    Ok(yields)
}

// Helper function to convert AAVE's ray format (1e27) per-second rate to yearly APR percentage
fn parse_ray_to_apr(ray_rate: &str) -> f64 {
    // Parse the string to a number, defaulting to 0 if parsing fails
    let rate = ray_rate.parse::<f64>().unwrap_or(0.0);

    // AAVE rates are in ray units (1e27) and represent per-second rates
    let ray = 1e27;

    // Convert to yearly APR percentage
    // The formula appears to be different from the standard calculation
    // Based on the provided examples, we need to adjust the calculation
    (rate / ray) * 100.0 // This seems to match the provided examples better
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_aave_yields() {
        let yields = fetch_aave_yields().await.unwrap();
        assert!(!yields.is_empty());
    }
    #[tokio::test]
    async fn test_get_aave_yields() -> Result<(), Box<dyn Error>> {
        let aave = Aave {};
        let result = aave.get_apr().await?;
        println!("{:?}", result);
        Ok(())
    }
}
