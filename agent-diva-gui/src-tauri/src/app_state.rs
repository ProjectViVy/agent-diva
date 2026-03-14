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

    pub async fn reconfigure(
        &self,
        api_base: Option<String>,
        api_key: Option<String>,
        provider: Option<String>,
        model: Option<String>,
    ) -> Result<(), String> {
        let url = format!("{}/config", self.api_base_url);
        let payload = serde_json::json!({
            "api_base": api_base,
            "api_key": api_key,
            "provider": provider,
            "model": model
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server error: {}", response.status()));
        }

        Ok(())
    }

    pub async fn get_tools_config(&self) -> Result<serde_json::Value, String> {
        let url = format!("{}/tools", self.api_base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Server error: {}", response.status()));
        }
        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Invalid JSON: {}", e))
    }

    pub async fn update_tools_config(&self, tools: serde_json::Value) -> Result<(), String> {
        let url = format!("{}/tools", self.api_base_url);
        let response = self
            .client
            .post(&url)
            .json(&tools)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Server error: {}", response.status()));
        }
        Ok(())
    }

    pub async fn get_skills(&self) -> Result<serde_json::Value, String> {
        let url = format!("{}/skills", self.api_base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Server error: {}", response.status()));
        }
        let value = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
            return Err(value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string());
        }
        Ok(value
            .get("skills")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])))
    }

    pub async fn get_mcps(&self) -> Result<serde_json::Value, String> {
        let url = format!("{}/mcps", self.api_base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Server error: {}", response.status()));
        }
        let value = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        if value.get("status").and_then(|v| v.as_str()) != Some("ok") {
            return Err(value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string());
        }
        Ok(value
            .get("mcps")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])))
    }
}
