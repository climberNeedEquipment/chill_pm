use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequest,
    },
    Client,
};
use async_trait::async_trait;
use reqwest::Client as ReqClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

// Define the Agent trait
#[async_trait]
pub trait Agent {
    fn set_prompt(&mut self, prompt: String) -> &mut Self;
    async fn chat(&self, messages: Vec<Message>) -> Result<String>;
    fn prompt(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct GaiaAIAgent {
    client: ReqClient,
    api_key: String,
    model: String,
    temperature: f32,
    prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GaiaRequest {
    messages: Vec<Message>,
    model: String,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct GaiaChoice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct GaiaResponse {
    choices: Vec<GaiaChoice>,
}

impl GaiaAIAgent {
    pub fn new(client: ReqClient, api_key: String, model: String, temperature: f32) -> Self {
        GaiaAIAgent {
            client,
            api_key,
            model,
            temperature,
            prompt: String::new(),
        }
    }
}

#[async_trait]
impl Agent for GaiaAIAgent {
    fn set_prompt(&mut self, prompt: String) -> &mut Self {
        self.prompt = prompt;
        self
    }

    fn prompt(&self) -> &str {
        &self.prompt
    }

    async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        use anyhow::anyhow;
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

        println!("Sending the following messages to Gaia AI:");
        for (i, msg) in messages.iter().enumerate() {
            println!(
                "  Message {}: role={}, content={}",
                i, msg.role, msg.content
            );
        }

        // Create headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );

        // Create request body - use the messages directly for Gaia API
        // Unlike OpenAI, we don't need to transform the messages format
        let request_body = GaiaRequest {
            messages: messages, // Use the messages as is
            model: self.model.clone(),
            temperature: self.temperature,
        };

        // Send request to Gaia API
        let response = self.client
            .post("https://0x09b36747b6f553b9ffe285c0c4e09fd5738f3245.gaia.domains/v1/chat/completions")
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        // Check if request was successful
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Gaia API error: {}", error_text));
        }

        // Parse response
        let gaia_response: GaiaResponse = response.json().await?;

        // Extract generated text
        if let Some(choice) = gaia_response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow!("No response from Gaia AI"))
        }
    }
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
        // Debug print all messages
        println!("Sending the following messages to OpenAI:");
        for (i, msg) in messages.iter().enumerate() {
            println!(
                "  Message {}: role={}, content={}",
                i, msg.role, msg.content
            );
        }
        let request_messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|msg| match msg.role.as_str() {
                "system" => {
                    ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                        content: ChatCompletionRequestSystemMessageContent::Text(msg.content),
                        name: None,
                    })
                }
                "assistant" => {
                    ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                        content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                            msg.content,
                        )),
                        name: None,
                        function_call: None,
                        tool_calls: None,
                        refusal: None,
                        audio: None,
                    })
                }
                _ => ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(msg.content),
                    name: None,
                }),
            })
            .collect();

        // Create the request
        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: request_messages,
            temperature: None,
            ..Default::default()
        };

        // Send the request
        let response = self.client.chat().create(request).await?;

        println!("Response: {:?}", response);

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
        prices: &String,
        portfolio_summary: &String,
    ) -> Result<String> {
        // Fetch the user's portfolio data for the specific token

        // Create a message asking for yield farming advice based on the portfolio
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: format!(
                    "I have the following portfolio:\n\n{}\n\n
                    Here is the current market price of the tokens in the portfolio:\n\n{}\n\n
                    I want to optimize my yield farming \
                    strategy. \n\n\
                    Please recommend a strategy that is delta neutral, meaning you should take both opposite positions between CEX and DEX. \
                    The Eisen portfoilio is for DEX, and Binance is for CEX.
                    In Binance, you can only trade on BTC and ETH
                    In Eisen, you can trade on all the tokens in the portfolio.
                    Here is an example of ouput format that should be in JSON format do not print anything else:\
                    {}",
                    portfolio_summary,
                    prices,
                    r#"
{
    "exchanges": [
        {
            "target": "Binance",
            "positions": [
                {
                    "position": "short",
                    "token": "<token_symbol1>",
                    "amount": "<amount>",
                    "price": "<price>",
                    "side": "sell"
                },
                {
                    "position": "short",
                    "token": "<token_symbol2>",
                    "amount": "<amount>",
                    "price": "<price>",
                    "side": "sell"
                }
            ]   
        },
        {
            "target": "Eisen",
            "swaps": [
                {
                    "position": "long",
                    "token": "<token_symbol1>",
                    "amount": "<amount>",
                },
                {
                    "position": "long",
                    "token": "<token_symbol2>",
                    "amount": "<amount>",
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
