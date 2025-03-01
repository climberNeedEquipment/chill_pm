use crate::portfolio::eisen::{
    get_onchain_portfolio, get_token_exposure_onchain, UserOnchainPortfolio,
};
use crate::utils::sign::BinanceKey;
use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequest, Role},
    Client,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// Define the Agent trait
#[async_trait]
pub trait Agent {
    fn set_prompt(&mut self, prompt: String) -> &mut Self;
    async fn chat(&self, messages: Vec<Message>) -> Result<String>;
    fn prompt(&self) -> &str;
}

pub struct OpenAIAgent {
    client: Client<OpenAIConfig>,
    model: String,
    temperature: f32,
    prompt: String,
}

impl OpenAIAgent {
    pub fn new(api_key: String, model: String, temperature: f32) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);

        Self {
            client,
            model,
            temperature,
            prompt: String::new(),
        }
    }
}

#[async_trait]
impl Agent for OpenAIAgent {
    fn set_prompt(&mut self, prompt: String) -> &mut Self {
        self.prompt = prompt;
        self
    }

    fn prompt(&self) -> &str {
        &self.prompt
    }

    async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        // Convert our Message type to the library's ChatCompletionRequestMessage type
        let request_messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|msg| {
                match msg.role.as_str() {
                    "system" => ChatCompletionRequestMessage::System(
                        async_openai::types::ChatCompletionRequestSystemMessage {
                            content: async_openai::types::ChatCompletionRequestSystemMessageContent::Text(msg.content),
                            name: None,
                        }
                    ),
                    _ => ChatCompletionRequestMessage::User(
                        async_openai::types::ChatCompletionRequestUserMessage {
                            content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(msg.content),
                            name: None,
                        }
                    ),
                }
            })
            .collect();

        // Create the request
        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: request_messages,
            temperature: Some(self.temperature),
            ..Default::default()
        };

        // Send the request
        let response = self.client.chat().create(request).await?;

        // Extract the response content
        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No completion choices returned"))?;

        Ok(choice.message.content.clone().unwrap_or_default())
    }
}

pub struct StableYieldFarmingAgent<A: Agent> {
    inner: A,
}

impl<A: Agent> StableYieldFarmingAgent<A> {
    pub fn new(mut agent: A) -> Self {
        // Set the specialized finance prompt
        agent.set_prompt(String::from(
            "You are a specialized financial advisor focused on stable yield farming strategies. \
            Provide conservative, well-researched advice on DeFi protocols, yield optimization, \
            risk assessment, and portfolio diversification. Always prioritize security and \
            sustainability over high APYs. Include relevant warnings about smart contract risks, \
            impermanent loss, and market volatility where appropriate.",
        ));

        Self { inner: agent }
    }

    // Delegate the chat method to the inner Agent
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        // Create a new vector with the system prompt as the first message
        let mut all_messages = vec![Message {
            role: "system".to_string(),
            content: self.inner.prompt().to_string(),
        }];

        // Add the user messages
        all_messages.extend(messages);

        // Call the inner agent's chat method
        self.inner.chat(all_messages).await
    }

    pub async fn get_portfolio_summary(
        &self,
        base_url: &str,
        wallet_address: &str,
        binance_key: &BinanceKey,
    ) -> Result<String> {
        // Fetch the user's on-chain portfolio data
        let onchain_portfolio = get_onchain_portfolio(base_url, wallet_address).await?;

        // Initialize portfolio data string - use debug format instead of JSON serialization
        let mut portfolio_data = format!("{:#?}", onchain_portfolio);

        // Fetch Binance portfolio data
        if let Ok(binance_account) = crate::portfolio::binance::get_binance_portfolio("https://fapi.binance.com", binance_key).await {
            let binance_data = format!("{:#?}", binance_account);
            portfolio_data.push_str("\n\n--- Binance Portfolio Data ---\n\n");
            portfolio_data.push_str(&binance_data);
        }

        // Format the portfolio data into a more readable tabular format
        let mut formatted_portfolio = String::new();

        // Format on-chain portfolio data
        formatted_portfolio.push_str("--- On-Chain Portfolio Summary ---\n\n");

        // Add chain details
        for chain in &onchain_portfolio.chain_details {
            formatted_portfolio.push_str(&format!("Chain ID: {}\n", chain.chain_id));

            // Add asset totals for this chain
            formatted_portfolio.push_str("Asset Totals:\n");
            for (asset, amount) in &chain.asset_total_amount_in_chain {
                formatted_portfolio.push_str(&format!("  {}: {:.6}\n", asset, amount));
            }

            // Add protocol details
            formatted_portfolio.push_str("\nProtocols:\n");
            for protocol in &chain.protocol_details {
                formatted_portfolio.push_str(&format!("  {}:\n", protocol.name));

                // Add assets for this protocol
                for asset in &protocol.assets {
                    formatted_portfolio.push_str(&format!(
                        "    {}: Amount={:?}, Underlying={:?}\n",
                        asset.symbol, 
                        asset.amount_to_calc_underlying, 
                        asset.underlying_balance
                    ));
                }
            }
            formatted_portfolio.push_str("\n");
        }

        // Add Binance data
        if let Ok(binance_account) = crate::portfolio::binance::get_binance_portfolio(
            "https://fapi.binance.com",
            binance_key,
        )
        .await
        {
            formatted_portfolio.push_str("--- Binance Portfolio Summary ---\n\n");
            formatted_portfolio.push_str(&format!(
                "Total Wallet Balance: {}\n\
                 Total Unrealized Profit: {}\n\
                 Total Margin Balance: {}\n",
                binance_account.total_wallet_balance,
                binance_account.total_unrealized_profit,
                binance_account.total_margin_balance
            ));

            // Add asset details
            formatted_portfolio.push_str("Assets:\n");
            for asset in &binance_account.assets {
                if asset.wallet_balance != "0" {
                    formatted_portfolio.push_str(&format!(
                        "  {}: Balance={}, Available={}\n",
                        asset.asset, asset.wallet_balance, asset.available_balance
                    ));
                }
            }

            // Add position details
            formatted_portfolio.push_str("\nPositions:\n");
            for position in &binance_account.positions {
                if position.position_amt != "0" {
                    formatted_portfolio.push_str(&format!(
                        "  {}: Amount={}, Side={}, Unrealized Profit={}\n",
                        position.symbol,
                        position.position_amt,
                        position.position_side,
                        position.unrealized_profit
                    ));
                }
            }
        }

        // Replace the raw JSON with the formatted tabular data
        portfolio_data = formatted_portfolio;
        Ok(portfolio_data)
    }

    pub async fn get_farming_strategy(
        &self,
        base_url: &str,
        wallet_address: &str,
        target_token: &str,
    ) -> Result<String> {
        // Fetch the user's portfolio data for the specific token
        let onchain_portfolio = get_onchain_portfolio(base_url, wallet_address).await?;
        let token_exposure = get_token_exposure_onchain(onchain_portfolio, target_token).await?;

        // Create a summary of the portfolio for the AI
        let portfolio_summary = self.create_portfolio_summary(&token_exposure, target_token);

        // Create a message asking for yield farming advice based on the portfolio
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: format!(
                    "I have the following {} portfolio:\n\n{}\n\nI want to optimize my yield farming \
                    strategy specifically for {}. \n\n\
                    Please recommend a strategy that is delta neutral, meaning you should take both opposite positions between CEX and DEX. \
                    The Eisen portfoilio is for DEX, and Binance is for CEX. \
                    Adjust your position in each exchange so that the portfolio results in delta neutral on native assets, but still has \
                    a yield from staking and restaking ETH tokens. \
                    Ouput format should be in JSON format in this format:\
                    {}",
                    target_token.to_uppercase(),
                    portfolio_summary,
                    target_token.to_uppercase(),
                    r#"
{
    "exchanges": [
        {
            "target": "Binance",
            "positions": [
                {
                    "position": "short",
                    "token": "ETH",
                    "amount": "100",
                    "price": "3000",
                    "side": "sell"
                },
                {
                    "position": "short",
                    "token": "ETH",
                    "amount": "100",
                    "price": "3000",
                    "side": "sell"
                }
            ]   
        },
        {
            "target": "Eisen",
            "positions": [
                {
                    "position": "long",
                    "token": "mETH",
                    "amount": "100",
                    "price": "3000",
                    "side": "buy"
                },
                {
                    "position": "long",
                    "token": "stETH",
                    "amount": "100",
                    "price": "3000",
                    "side": "buy"
                }
            ]
        }
    ]
}
                    "#
                ),
            },
        ];

        // Get the AI's recommendation
        self.chat(messages).await
    }

    fn create_portfolio_summary(&self, portfolio: &UserOnchainPortfolio, token: &str) -> String {
        let mut summary = format!(
            "Total {} exposure: {:.4} {}\n\n",
            token.to_uppercase(),
            portfolio.total_exposure,
            token.to_uppercase()
        );

        summary.push_str("Current holdings breakdown:\n");

        for chain in &portfolio.chain_details {
            // Add chain name based on chain ID if available
            let chain_name = match chain.chain_id {
                1 => "Ethereum",
                56 => "BSC",
                137 => "Polygon",
                42161 => "Arbitrum",
                10 => "Optimism",
                // Add more chains as needed
                _ => "Unknown Chain",
            };

            summary.push_str(&format!(
                "\n{} (Chain ID: {}):\n",
                chain_name, chain.chain_id
            ));

            for protocol in &chain.protocol_details {
                summary.push_str(&format!("  Protocol: {}\n", protocol.name));

                for asset in &protocol.assets {
                    summary.push_str(&format!(
                        "    - {} {:?} (underlying amount: {:?})\n",
                        asset.symbol, 
                        asset.balance, 
                        asset.underlying_amount
                    ));
                }
            }
        }
        summary
    }
}
