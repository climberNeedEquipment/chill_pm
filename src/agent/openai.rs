use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, CreateChatCompletionRequest,
        ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    },
    Client,
};
use async_trait::async_trait;

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
                        ChatCompletionRequestSystemMessage {
                            content: ChatCompletionRequestSystemMessageContent::Text(msg.content),
                            name: None,
                        }
                    ),
                    "assistant" => ChatCompletionRequestMessage::Assistant(
                        ChatCompletionRequestAssistantMessage {
                            content: Some(ChatCompletionRequestAssistantMessageContent::Text(msg.content)),
                            name: None,
                            function_call: None,
                            tool_calls: None,
                            refusal: None,
                            audio: None,
                        }
                    ),
                    _ => ChatCompletionRequestMessage::User(
                        ChatCompletionRequestUserMessage {
                            content: ChatCompletionRequestUserMessageContent::Text(msg.content),
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

    pub async fn get_farming_strategy(
        &self,
        portfolio_summary: &String
    ) -> Result<String> {
        // Fetch the user's portfolio data for the specific token

        // Create a message asking for yield farming advice based on the portfolio
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: format!(
                    "I have the following portfolio:\n\n{}\n\nI want to optimize my yield farming \
                    strategy. \n\n\
                    Please recommend a strategy that is delta neutral, meaning you should take both opposite positions between CEX and DEX. \
                    The Eisen portfoilio is for DEX, and Binance is for CEX. \
                    Adjust your position in each exchange so that the portfolio results in delta neutral on native assets, but still has \
                    a yield from staking and restaking ETH tokens. \
                    Ouput format should be in JSON format in this format and do not print anything else:\
                    {}",
                    portfolio_summary,
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
}
