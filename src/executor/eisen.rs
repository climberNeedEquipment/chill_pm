use alloy::network::TransactionBuilder;
use alloy::primitives::FixedBytes;
use alloy::primitives::{Address, Bytes, U256};
use alloy::providers::Provider;
use alloy::rpc::types::TransactionRequest;
use anyhow::Result;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BalanceAllowResponse {
    pub result: Vec<BalanceAllow>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BalanceAllow {
    pub token_address: String,
    pub balance: String,
    //allowance: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainMetadataResponse {
    result: ChainMetadata,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainMetadata {
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
pub struct ChainData {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    symbol: String,
    balance: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainPortfolio {
    pub balances: Vec<TokenBalance>,
}

pub async fn fetch_chain_portfolio(
    base_url: &str,
    chain_id: u64,
    wallet_addr: &String,
) -> Result<ChainPortfolio> {
    let url = format!(
        "{}/chains/{}/balances?walletAddress={}",
        base_url, chain_id, wallet_addr
    );
    let client = Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch chain metadata: HTTP {}",
            response.status()
        ));
    }

    let metadata: BalanceAllowResponse = response.json().await?;
    let chain_metadata = get_chain_metadata(base_url, chain_id).await?;
    let balance_allow = metadata
        .result
        .iter()
        .filter(|token| token.balance != "0")
        .map(|token| {
            let symbol = chain_metadata
                .addr_to_sym
                .get(token.token_address.as_str())
                .unwrap();
            let decimals = chain_metadata.sym_to_addr_n_decimals.get(symbol).unwrap().1;
            TokenBalance {
                symbol: symbol.to_string(),
                balance: token.balance.parse::<f64>().unwrap() / 10.0_f64.powi(decimals as i32),
            }
        })
        .collect();
    Ok(ChainPortfolio {
        balances: balance_allow,
    })
}

pub async fn get_chain_metadata(base_url: &str, chain_id: u64) -> Result<ChainData> {
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

pub async fn send_tx(
    provider: &dyn Provider,
    build_response: BuildResponse,
) -> Result<FixedBytes<32>> {
    let tx = TransactionRequest::default()
        .with_to(build_response.result.to)
        .with_value(build_response.result.value)
        .with_input(build_response.result.data);
    let receipt = provider.send_transaction(tx).await?.watch().await?;
    Ok(receipt)
}

pub async fn quote_and_send_tx(
    provider: &dyn Provider,
    base_url: &str,
    chain_data: &ChainData,
    from_token: &str,
    to_token: &str,
    amount: f64,
    wallet_addr: &Address,
    slippage_bps: u16,
) -> Result<FixedBytes<32>> {
    let chain_id = provider.get_chain_id().await?;

    let (src_token_addr, src_token_decimals) =
        &chain_data.sym_to_addr_n_decimals[&from_token.to_lowercase()];
    let (dst_token_addr, dst_token_decimals) =
        &chain_data.sym_to_addr_n_decimals[&to_token.to_lowercase()];

    let amount_in = U256::from_str_radix(
        &((amount * 10.0_f64.powi(*src_token_decimals as i32))
            .floor()
            .to_string()),
        10,
    )
    .unwrap();

    let quote = get_quote(
        base_url,
        chain_id,
        src_token_addr,
        dst_token_addr,
        amount_in,
        None,
    )
    .await?;

    let tx_data = get_tx_data(
        base_url,
        chain_id,
        quote.result.dex_agg.unwrap(),
        None,
        String::new(),
        wallet_addr.to_string().as_str(),
        slippage_bps,
    )
    .await?;

    let tx = send_tx(provider, tx_data).await?;

    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::network::EthereumWallet;
    use alloy::signers::local::PrivateKeySigner;
    use reqwest::Url;
    use tokio;

    use alloy::{
        network::{TransactionBuilder, TransactionResponse},
        primitives::U256,
        providers::{Provider, ProviderBuilder},
        rpc::types::TransactionRequest,
    };
    #[test]
    fn cmp_amount() -> Result<()> {
        let amount = 1.1;
        let src_token_decimals = 6;
        let amount_in = U256::from_str_radix(
            &((amount * 10.0_f64.powi(src_token_decimals))
                .floor()
                .to_string()),
            10,
        )
        .unwrap();

        println!("amount_in: {}", amount_in);
        assert_eq!(amount_in, U256::from_str_radix("1100000", 10).unwrap());
        Ok(())
    }
    use dotenv::dotenv;
    use std::env;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_chain_metadata() -> Result<()> {
        dotenv().unwrap();

        let signer: PrivateKeySigner = env::var("PRIVATE_KEY_DEPLOYER")
            .expect("PRIVATE_KEY must be set in .env")
            .chars()
            .skip(2)
            .collect::<String>()
            .parse()
            .unwrap();

        let wallet = EthereumWallet::from(signer);

        println!("{:?}", wallet.default_signer().address());

        // Mock base URL and chain ID
        let base_url = env::var("EISEN_BASE_URL").expect("EISEN_BASE_URL must be set in .env");
        let rpc_url = Url::parse("https://mainnet.base.org").unwrap();
        let provider = ProviderBuilder::new()
            .wallet(wallet.clone())
            .on_http(rpc_url);

        let provider = Arc::new(provider);
        let chain_id = provider.get_chain_id().await?;

        // Call the function
        let result = get_chain_metadata(&base_url, chain_id).await?;
        let src_token = "eth";
        let dst_token = "weeth";

        let (src_token_addr, src_token_decimals) =
            &result.sym_to_addr_n_decimals[&src_token.to_lowercase()];
        let (dst_token_addr, dst_token_decimals) =
            &result.sym_to_addr_n_decimals[&dst_token.to_lowercase()];
        let amount_in = U256::from_str_radix("1000000000000000", 10).unwrap();
        let quote = get_quote(
            &base_url,
            chain_id,
            src_token_addr,
            dst_token_addr,
            amount_in,
            None,
        )
        .await?;

        let addr = "0xdAf87a186345f26d107d000fAD351E79Ff696d2C".to_string();

        let tx_data = get_tx_data(
            &base_url,
            chain_id,
            quote.result.dex_agg.unwrap(),
            None,
            String::new(),
            &addr,
            100,
        )
        .await?;

        let rpc_url = Url::parse("https://base.llamarpc.com").unwrap();

        // let anvil = Anvil::new()
        //     .fork(rpc_url)
        //     .fork_block_number(fork_block)
        //     .block_time(1_u64)
        //     .timeout(60_u64)
        //     .spawn();

        // let anvil_provider = ProviderBuilder::new()
        //     .wallet(wallet)
        //     .on_http(anvil.endpoint().parse().unwrap());
        // let anvil_provider = Arc::new(anvil_provider);

        let call_data = tx_data.result.data;

        let tx = TransactionRequest::default()
            .with_to(tx_data.result.to)
            .with_value(tx_data.result.value)
            .with_input(call_data);

        let receipt = provider.send_transaction(tx).await?.watch().await?;
        println!("Sent transaction: {:?}", receipt);

        Ok(())
    }
}
