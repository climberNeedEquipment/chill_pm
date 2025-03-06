use crate::agent::Strategy;
use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde_json;
pub struct OthenticAgent {
    host: String,
    port: u16,
    client: ReqwestClient,
    task_definition_id: String,
}

impl OthenticAgent {
    pub fn new(host: String, port: u16, task_definition_id: Option<String>) -> Self {
        let client = ReqwestClient::new();
        Self {
            host,
            port,
            client,
            task_definition_id: task_definition_id.unwrap_or("0".to_string()),
        }
    }

    pub async fn get_strategy(
        &self,
        model: &String,
        price: &String,
        portfolio: &String,
    ) -> Result<Strategy> {
        let url = format!("http://{}:{}/task/execute", self.host, self.port);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "model": model,
                "price": price,
                "portfolio": portfolio,
                "taskDefinitionId": self.task_definition_id
            }))
            .send()
            .await?;

        let strategy = response.text().await?;
        // Parse the strategy string to JSON
        let strategy_json: serde_json::Value = serde_json::from_str(&strategy)?;

        let strategy = strategy_json
            .get("data")
            .and_then(|d| d.get("strategy"))
            .and_then(|s| s.as_str())
            .unwrap_or("No strategy found");
        // Parse the strategy string to a Strategy struct
        println!("Strategy from the agent:\n{}", strategy);
        let strategy_struct: Strategy = serde_json::from_str(strategy)
            .map_err(|e| anyhow::anyhow!("Failed to parse strategy: {}", e))?;

        // Pretty print the strategy struct as JSON
        let pretty_json = serde_json::to_string_pretty(&strategy_struct)
            .map_err(|e| anyhow::anyhow!("Failed to serialize strategy to pretty JSON: {}", e))?;
        
        println!("Strategy as pretty JSON:\n{}", pretty_json);
        // Return the original JSON value
        Ok(strategy_struct)
    }
}
