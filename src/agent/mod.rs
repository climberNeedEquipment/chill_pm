pub mod openai;
pub mod othentic;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Define the Agent trait
#[async_trait]
pub trait Agent {
    fn set_prompt(&mut self, prompt: String) -> &mut Self;
    async fn chat(&self, messages: Vec<Message>) -> Result<String>;
    fn prompt(&self) -> &str;
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}
