use agent_diva_providers::{
    CustomProviderUpsert, ProviderModelCatalogView as SharedProviderModelCatalog, ProviderView,
};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AgentState {
    pub client: reqwest::Client,
    api_base_url: Arc<RwLock<String>>,
    gateway_port: Arc<RwLock<u16>>,
}

impl AgentState {
    pub fn new() -> Self {
        // Try to load the gateway port from config, defaults to 3000
        let gateway_port = Self::load_gateway_port();
        Self {
            // Local Manager only: never use the system HTTP proxy (common on Windows with VPN /
            // corporate proxy). Proxying 127.0.0.1 often yields 502 Bad Gateway from the proxy.
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("reqwest client for local Manager API"),
            // Must match `agent-diva-manager` bind (`127.0.0.1` only). Using `localhost` can
            // resolve to `::1` first on Windows; nothing listens there → health checks stay offline.
            api_base_url: Arc::new(RwLock::new(Self::api_base_url_for_port(gateway_port))),
            gateway_port: Arc::new(RwLock::new(gateway_port)),
        }
    }

    fn api_base_url_for_port(port: u16) -> String {
        format!("http://127.0.0.1:{port}/api")
    }

    pub fn api_base_url(&self) -> String {
        self.api_base_url
            .read()
            .map(|value| value.clone())
            .unwrap_or_else(|_| Self::api_base_url_for_port(3000))
    }

    pub fn update_gateway_port(&self, gateway_port: u16) {
        if let Ok(mut guard) = self.gateway_port.write() {
            *guard = gateway_port;
        }
        if let Ok(mut guard) = self.api_base_url.write() {
            *guard = Self::api_base_url_for_port(gateway_port);
        }
    }

    /// Load gateway port from config file, defaults to 3000
    fn load_gateway_port() -> u16 {
        if let Ok(config_dir) = std::env::var("AGENT_DIVA_CONFIG_DIR") {
            if !config_dir.trim().is_empty() {
                let port_file = std::path::PathBuf::from(config_dir).join("gateway.port");
                if let Ok(content) = std::fs::read_to_string(&port_file) {
                    if let Ok(port) = content.trim().parse::<u16>() {
                        return port;
                    }
                }
            }
        }

        // Try default config directory location
        let loader = agent_diva_core::config::ConfigLoader::new();
        let port_file = loader.config_dir().join("gateway.port");
        if let Ok(content) = std::fs::read_to_string(&port_file) {
            if let Ok(port) = content.trim().parse::<u16>() {
                return port;
            }
        }

        // Fallback to default port
        3000
    }

    pub async fn reconfigure(
        &self,
        api_base: Option<String>,
        api_key: Option<String>,
        provider: Option<String>,
        model: Option<String>,
    ) -> Result<(), String> {
        let url = format!("{}/config", self.api_base_url());
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
        let url = format!("{}/tools", self.api_base_url());
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
        let url = format!("{}/tools", self.api_base_url());
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

    pub async fn get_provider_views(&self) -> Result<Vec<ProviderView>, String> {
        let url = format!("{}/providers", self.api_base_url());
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
            .json::<Vec<ProviderView>>()
            .await
            .map_err(|e| format!("Invalid JSON: {}", e))
    }

    pub async fn get_provider_model_catalog(
        &self,
        provider: &str,
        runtime: bool,
    ) -> Result<SharedProviderModelCatalog, String> {
        let url = format!(
            "{}/providers/{}/models?runtime={}",
            self.api_base_url(),
            urlencoding::encode(provider),
            runtime
        );
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
            .json::<SharedProviderModelCatalog>()
            .await
            .map_err(|e| format!("Invalid JSON: {}", e))
    }

    pub async fn create_custom_provider(
        &self,
        payload: &CustomProviderUpsert,
    ) -> Result<Option<ProviderView>, String> {
        let url = format!("{}/providers", self.api_base_url());
        let response = self
            .client
            .post(&url)
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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
        serde_json::from_value(
            value
                .get("provider")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )
        .map_err(|e| format!("Invalid provider payload: {}", e))
    }

    pub async fn delete_custom_provider(&self, provider: &str) -> Result<(), String> {
        let url = format!(
            "{}/providers/{}",
            self.api_base_url(),
            urlencoding::encode(provider)
        );
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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
        Ok(())
    }

    pub async fn add_provider_model(&self, provider: &str, model: &str) -> Result<(), String> {
        let url = format!(
            "{}/providers/{}/models",
            self.api_base_url(),
            urlencoding::encode(provider)
        );
        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "model": model }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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
        Ok(())
    }

    pub async fn remove_provider_model(&self, provider: &str, model: &str) -> Result<(), String> {
        let url = format!(
            "{}/providers/{}/models/{}",
            self.api_base_url(),
            urlencoding::encode(provider),
            urlencoding::encode(model)
        );
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
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
        Ok(())
    }

    pub async fn get_skills(&self) -> Result<serde_json::Value, String> {
        let url = format!("{}/skills", self.api_base_url());
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
        let url = format!("{}/mcps", self.api_base_url());
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
