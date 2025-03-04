use crate::agent::Agent;
use crate::agent::Message;
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
