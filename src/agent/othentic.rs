use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde_json;
use crate::agent::Strategy;
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
        let strategy_struct: Strategy = serde_json::from_str(strategy)
            .map_err(|e| anyhow::anyhow!("Failed to parse strategy: {}", e))?;
        
        // Return the original JSON value
        Ok(strategy_struct)
    }
}
