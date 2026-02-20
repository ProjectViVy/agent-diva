use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AgentState {
    pub client: reqwest::Client,
    pub api_base_url: String,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base_url: "http://localhost:3000/api".to_string(),
        }
    }
    
    pub async fn reconfigure(&self, api_base: Option<String>, api_key: Option<String>, model: Option<String>) -> Result<(), String> {
        let url = format!("{}/config", self.api_base_url);
        let payload = serde_json::json!({
            "api_base": api_base,
            "api_key": api_key,
            "model": model
        });
        
        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
            
        if !response.status().is_success() {
             return Err(format!("Server error: {}", response.status()));
        }
        
        Ok(())
    }
}
