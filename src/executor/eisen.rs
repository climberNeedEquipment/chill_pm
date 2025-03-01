use alloy::primitives::{Address, Bytes, U256};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChainMetadataResponse {
    result: ChainMetadata,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChainMetadata {
    id: String,
    native_symbol: String,
    tokens: Vec<Token>,
}

#[derive(Deserialize, Debug)]
struct Token {
    address: String,
    symbol: String,
    decimals: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChainData {
    id: u64,
    name: String,
    sym_to_addr_n_decimals: HashMap<String, (String, u8)>,
    addr_to_sym: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequestBody {
    token_in_addr: String,
    token_out_addr: String,
    amount: String,
    max_split: String,
    max_edge: String,
    with_cycle: bool,
    dex_id_filter: Vec<String>,
    custom_tokens: Option<String>,
    from: Option<String>,
}

#[derive(Deserialize, Debug)]
struct QuoteResponse {
    result: QuoteResult,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResult {
    is_swap_path_exists: bool,
    dex_agg: Option<AggregateMergeSwapInfo>,
    cexes: Vec<Cex>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cex {
    cex_id: String,
    amount_in: String,
    expected_amount_out: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateMergeSwapInfo {
    pub block_number: u64,
    pub from_token: String,
    pub amount_in: String,
    pub to_token: String,
    pub weights: Vec<u16>,
    pub total_addrs: Vec<String>,
    pub src_indices: Vec<u16>,
    pub dst_indices: Vec<u16>,
    pub split_infos: Vec<MergeSplitPathInfo>,
    pub expected_amount_out: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeSplitPathInfo {
    pub src_idx: u16,
    pub dst_idx: u16,
    pub weight: u16,
    pub total_weights: u16,
    pub swap_info: SingleSwapInfo,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleSwapInfo {
    pub from_token: String,
    pub to_token: String,
    pub dex_id: String,
    pub pool: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRequestBody {
    from: String,
    slippage_bps: String,
    permit: Option<PermitSingle>,
    permit_signature: String,
    dex_agg: AggregateMergeSwapInfo,
    cycles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermitSingle {
    details: PermitDetails,
    spender: String,
    sig_deadline: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermitDetails {
    token: String,
    amount: String,
    expiration: u64,
    nonce: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    result: Transaction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    from: Address,
    to: Address,
    value: U256,
    data: Bytes,
    gas_limit: u64,
    estimated_gas: u64,
    error: Option<String>,
}

fn convert_chain_id_to_name(chain_id: u64) -> String {
    match chain_id {
        1 => "mainnet".to_string(),
        8453 => "base".to_string(),
        34443 => "mode".to_string(),
        _ => "Unknown".to_string(),
    }
}

async fn get_chain_metadata(base_url: &str, chain_id: u64) -> Result<ChainData> {
    let url = format!("{}/chains/{}/metadata", base_url, chain_id);
    let client = Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch chain metadata: HTTP {}",
            response.status()
        ));
    }

    let metadata: ChainMetadataResponse = response.json().await?;

    let chain_data = ChainData {
        id: metadata.result.id.parse::<u64>()?,
        name: convert_chain_id_to_name(metadata.result.id.parse::<u64>()?),
        sym_to_addr_n_decimals: metadata
            .result
            .tokens
            .iter()
            .map(|token| {
                (
                    token.symbol.to_lowercase(),
                    (token.address.to_lowercase(), token.decimals),
                )
            })
            .collect(),
        addr_to_sym: metadata
            .result
            .tokens
            .iter()
            .map(|token| (token.address.to_lowercase(), token.symbol.to_lowercase()))
            .collect(),
    };

    Ok(chain_data)
}

pub async fn get_quote(
    base_url: &str,
    chain_id: u64,
    from_token: &str,
    to_token: &str,
    amount: U256,
    from: Option<String>,
) -> Result<QuoteResponse> {
    let url = format!("{}/chains/{}/v2/quote", base_url, chain_id);
    let client = Client::new();

    let quote_request_body = QuoteRequestBody {
        token_in_addr: from_token.to_string(),
        token_out_addr: to_token.to_string(),
        amount: amount.to_string(),
        max_split: "10".to_string(),
        max_edge: "3".to_string(),
        with_cycle: false,
        dex_id_filter: vec![],
        custom_tokens: None,
        from,
    };

    let response = client
        .post(url)
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&quote_request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch quote: HTTP {}",
            response.status()
        ));
    }

    let quote_response: QuoteResponse = response.json().await?;

    Ok(quote_response)
}

pub async fn get_tx_data(
    base_url: &str,
    chain_id: u64,
    dex_agg: AggregateMergeSwapInfo,
    permit: Option<PermitSingle>,
    permit_signature: String,
    from: &str,
    slippage_bps: u16,
) -> Result<BuildResponse> {
    let url = format!("{}/chains/{}/v2/build", base_url, chain_id);
    let client = Client::new();

    let build_request_body = BuildRequestBody {
        from: from.to_string(),
        slippage_bps: slippage_bps.to_string(),
        permit,
        permit_signature,
        dex_agg,
        cycles: vec![],
    };

    let response = client
        .post(url)
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&build_request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch quote: HTTP {}",
            response.status()
        ));
    }

    let build_response: BuildResponse = response.json().await?;

    Ok(build_response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use dotenv::dotenv;
    use std::env;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_chain_metadata() -> Result<()> {
        dotenv().unwrap();

        // Mock base URL and chain ID
        let base_url = env::var("EISEN_BASE_URL").expect("EISEN_BASE_URL must be set in .env");
        let chain_id = 8453;

        // Call the function
        let result = get_chain_metadata(&base_url, chain_id).await?;
        let src_token = "eth";
        let dst_token = "usdc";

        let (src_token_addr, src_token_decimals) =
            &result.sym_to_addr_n_decimals[&src_token.to_lowercase()];
        let (dst_token_addr, dst_token_decimals) =
            &result.sym_to_addr_n_decimals[&dst_token.to_lowercase()];

        let quote = get_quote(
            &base_url,
            chain_id,
            src_token_addr,
            dst_token_addr,
            U256::from_str_radix("1000000000000000", 10).unwrap(),
            None,
        )
        .await?;

        let tx_data = get_tx_data(
            &base_url,
            chain_id,
            quote.result.dex_agg.unwrap(),
            None,
            String::new(),
            "0xdAf87a186345f26d107d000fAD351E79Ff696d2C",
            100,
        )
        .await?;

        Ok(())
    }
}
