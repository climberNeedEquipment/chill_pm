use anyhow::Result;
use async_openai::{
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequest, Role},
    Client,
};
use async_trait::async_trait;
use crate::portfolio::eisen::{get_onchain_portfolio, get_token_exposure_onchain, UserOnchainPortfolio};
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
    client: Client,
    model: String,
    temperature: f32,
    prompt: String,
}

impl OpenAIAgent {
    pub fn new(api_key: String, model: String, temperature: f32) -> Self {
        let client = Client::new().with_api_key(api_key);
        
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
            .map(|msg| ChatCompletionRequestMessage {
                role: match msg.role.as_str() {
                    "system" => Role::System,
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    _ => Role::User, // Default to user for unknown roles
                },
                content: msg.content,
                name: None,
                function_call: None,
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
        let choice = response.choices.first()
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
            impermanent loss, and market volatility where appropriate."
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

    pub async fn get_farming_strategy(&self, base_url: &str, wallet_address: &str, target_token: &str) -> Result<String> {
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
                    strategy specifically for {}. Please suggest how I should rebalance my portfolio \
                    to maximize stable yields for this asset while minimizing risk. Include specific \
                    protocols that offer good {} yields, estimated APYs, and any relevant warnings \
                    about the recommended platforms.",
                    target_token.to_uppercase(),
                    portfolio_summary,
                    target_token.to_uppercase(),
                    target_token.to_uppercase()
                ),
            },
        ];
        
        // Get the AI's recommendation
        self.chat(messages).await
    }
    
    fn create_portfolio_summary(&self, portfolio: &UserOnchainPortfolio, token: &str) -> String {
        let mut summary = format!("Total {} exposure: {:.4} {}\n\n", 
            token.to_uppercase(), portfolio.total_exposure, token.to_uppercase());
            
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
            
            summary.push_str(&format!("\n{} (Chain ID: {}):\n", chain_name, chain.chain_id));
            
            for protocol in &chain.protocol_details {
                summary.push_str(&format!("  Protocol: {}\n", protocol.name));
                
                for asset in &protocol.assets {
                    summary.push_str(&format!("    - {} {:.4} (underlying amount: {:.4})\n", 
                        asset.symbol, asset.balance, asset.underlying_amount));
                }
            }
        }
        
        summary
    }
}

