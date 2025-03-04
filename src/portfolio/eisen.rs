use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnderlyingBalancesResponse {
    /// Maps asset symbols (e.g., "ETH", "BTC") to their total float amounts.
    #[serde(default)]
    pub asset_total_amount: HashMap<String, f64>,

    /// List of chains and their corresponding assets/protocols.
    #[serde(default)]
    pub chain_details: Vec<ChainDetail>,
}

impl std::fmt::Display for UnderlyingBalancesResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Portfolio Summary:")?;

        // Display chain details
        if !self.chain_details.is_empty() {
            writeln!(f, "\nChain Details:")?;
            for chain in &self.chain_details {
                if chain.chain_id != 8453 {
                    continue;
                }
                writeln!(f, "  Chain ID: {}", chain.chain_id)?;

                // Display assets in this chain
                if !chain.asset_total_amount_in_chain.is_empty() {
                    writeln!(f, "    Assets in Chain:")?;
                    for (symbol, amount) in &chain.asset_total_amount_in_chain {
                        writeln!(f, "      {}: {:.6}", symbol, amount)?;
                    }
                }

                // Display protocols in this chain
                if !chain.protocol_details.is_empty() {
                    writeln!(f, "    Protocols:")?;
                    for protocol in &chain.protocol_details {
                        writeln!(f, "      {}", protocol.name)?;

                        // Display assets in this protocol
                        if !protocol.assets.is_empty() {
                            writeln!(f, "        Assets:")?;
                            for asset in &protocol.assets {
                                writeln!(
                                    f,
                                    "          {} ({})",
                                    asset.symbol, asset.contract_address
                                )?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainDetail {
    /// Chain ID (e.g., 1 for Ethereum mainnet, 56 for BSC, etc.)
    pub chain_id: u64,

    /// Asset totals on this specific chain.
    #[serde(default)]
    pub asset_total_amount_in_chain: HashMap<String, f64>,

    /// Protocols available on this chain.
    #[serde(default)]
    pub protocol_details: Vec<ProtocolDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolDetail {
    /// Protocol name (e.g., "native", "Lido", "AaveV3").
    pub name: String,

    /// Optional URL to the protocol’s logo.
    #[serde(default)]
    pub logo_url: Option<String>,

    /// List of assets under this protocol.
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// Contract address of the asset (could be "0x000..." for native).
    pub contract_address: String,

    /// Symbol of the asset (e.g., "ETH", "WBTC").
    pub symbol: String,

    /// Optional URL to the asset’s icon.
    #[serde(default)]
    pub icon_url: Option<String>,

    /// Base contract address (for derivatives, wrappers, etc.).
    pub base_contract_address: String,

    /// Amount used for calculating underlying assets.
    pub amount_to_calc_underlying: Balance,

    /// The final underlying balance after calculations.
    pub underlying_balance: Balance,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    /// The raw amount as a string (e.g., "5162992717092596").
    pub amount: String,

    /// The decimal places to interpret `amount` (e.g., 18 for ETH).
    pub decimals: u8,

    /// Sign of the balance (true if positive).
    pub positive_sign: bool,
}

impl Balance {
    fn to_f64(&self) -> Result<f64> {
        let amount = f64::from_str(&self.amount).map_err(|_| anyhow!("invalid amount"))?
            / 10_f64.powi(self.decimals as i32);
        if self.positive_sign {
            Ok(amount)
        } else {
            Ok(-amount)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserOnchainPortfolio {
    pub total_exposure: f64,
    pub chain_details: Vec<ChainDetailFeed>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainDetailFeed {
    pub chain_id: u64,
    pub protocol_details: Vec<ProtocolDetailFeed>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolDetailFeed {
    pub name: String,
    pub assets: Vec<AssetFeed>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetFeed {
    pub symbol: String,
    pub balance: f64,
    pub underlying_amount: f64,
}
pub async fn get_onchain_portfolio(
    base_url: &str,
    wallet_address: &str,
) -> Result<UnderlyingBalancesResponse> {
    // Get base URL from environment variables

    // Construct the endpoint URL
    let url = format!(
        "{}/underlying_balances?walletAddress={}",
        base_url, wallet_address
    );

    // Create an async HTTP client
    let client = reqwest::Client::new();

    // Send the GET request
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|_| anyhow!("get request err"))?;
    let data: UnderlyingBalancesResponse = response.json().await?;

    Ok(data)
}

// New function to fetch underlying balances.
pub async fn get_token_exposure_onchain(
    data: UnderlyingBalancesResponse,
    token: &str,
) -> Result<UserOnchainPortfolio> {
    let base_addr = convert_sym_to_mapped_config_addr(token)?;

    // Deserialize JSON into our structs

    let user_onchain_portfolio = UserOnchainPortfolio {
        total_exposure: data
            .chain_details
            .iter()
            .map(|chain_detail| {
                chain_detail
                    .asset_total_amount_in_chain
                    .iter()
                    .filter_map(|(asset, amount)| {
                        if asset.to_lowercase() == token {
                            Some(*amount)
                        } else {
                            None
                        }
                    })
                    .fold(0 as f64, |sum, ele| sum + ele)
            })
            .sum(),
        chain_details: data
            .chain_details
            .iter()
            .map(|chain_detail| ChainDetailFeed {
                chain_id: chain_detail.chain_id,
                protocol_details: chain_detail
                    .protocol_details
                    .iter()
                    .map(|protocol_detail| ProtocolDetailFeed {
                        name: protocol_detail.name.clone(),
                        assets: protocol_detail
                            .assets
                            .iter()
                            .filter(|asset| {
                                asset.base_contract_address == base_addr
                                    && asset.underlying_balance.amount != "0"
                            })
                            .filter_map(|asset| {
                                let underlying_amount = match asset.underlying_balance.to_f64() {
                                    Ok(value) => value,
                                    Err(_) => return None,
                                };
                                let balance = match asset.amount_to_calc_underlying.to_f64() {
                                    Ok(value) => value,
                                    Err(_) => return None,
                                };
                                Some(AssetFeed {
                                    balance,
                                    symbol: asset.symbol.clone(),
                                    underlying_amount,
                                })
                            })
                            .collect::<Vec<_>>(),
                    })
                    .filter(|protocol_detail| !protocol_detail.assets.is_empty())
                    .collect::<Vec<_>>(),
            })
            .filter(|chain_detail| !chain_detail.protocol_details.is_empty())
            .collect::<Vec<_>>(),
    };

    Ok(user_onchain_portfolio)
}

fn convert_sym_to_mapped_config_addr(token_symbol: &str) -> Result<String> {
    match token_symbol.to_string().to_lowercase().as_str() {
        "eth" => Ok("0x0000000000000000000000000000000000000000".to_string()),
        "btc" => Ok("0x0000000000000000000000000000000000000001".to_string()),
        &_ => Err(anyhow!("Noonchain_portfoliod token")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use std::env;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_token_exposure_onchain() -> Result<()> {
        dotenv().unwrap();
        let base_url = env::var("EISEN_BASE_URL").expect("EISEN_BASE_URL must be set in .env");
        let wallet_address = "0xdAf87a186345f26d107d000fAD351E79Ff696d2C";
        let token = "eth";
        let onchain_portfolio = get_onchain_portfolio(base_url.as_str(), wallet_address).await?;
        let onchain_portfolio = get_token_exposure_onchain(onchain_portfolio, token)
            .await
            .map_err(|_| anyhow!("error getting onchain portfolio"))?;
        println!("{:?}", onchain_portfolio);
        Ok(())
    }
}
