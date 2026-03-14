use agent_diva_core::config::schema::{Config, MCPServerConfig};
use agent_diva_core::config::ConfigLoader;
use agent_diva_tools::probe_mcp_server;
use anyhow::anyhow;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpConnectionStatusDto {
    pub state: String,
    pub connected: bool,
    pub applied: bool,
    pub tool_count: usize,
    pub error: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerDto {
    pub name: String,
    pub enabled: bool,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: String,
    pub tool_timeout: u64,
    pub status: McpConnectionStatusDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerUpsert {
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout: u64,
}

fn default_tool_timeout() -> u64 {
    30
}

#[derive(Clone)]
pub struct McpService {
    loader: ConfigLoader,
}

impl McpService {
    pub fn new(loader: ConfigLoader) -> Self {
        Self { loader }
    }

    pub fn list_mcps(&self) -> anyhow::Result<Vec<McpServerDto>> {
        let config = self.loader.load()?;
        let mut list = config
            .tools
            .mcp_servers
            .iter()
            .map(|(name, server)| self.to_dto(&config, name, server))
            .collect::<Vec<_>>();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    pub fn create_mcp(&self, payload: McpServerUpsert) -> anyhow::Result<McpServerDto> {
        let mut config = self.loader.load()?;
        self.validate_name(&payload.name)?;
        if config.tools.mcp_servers.contains_key(&payload.name) {
            return Err(anyhow!("MCP '{}' already exists", payload.name));
        }

        let name = payload.name.clone();
        let enabled = payload.enabled;
        config
            .tools
            .mcp_servers
            .insert(name.clone(), Self::payload_to_config(&payload)?);
        Self::set_enabled_flag(&mut config, &name, enabled);
        self.loader.save(&config)?;
        self.get_mcp(&name)
    }

    pub fn update_mcp(
        &self,
        current_name: &str,
        payload: McpServerUpsert,
    ) -> anyhow::Result<McpServerDto> {
        let mut config = self.loader.load()?;
        self.validate_name(&payload.name)?;
        if !config.tools.mcp_servers.contains_key(current_name) {
            return Err(anyhow!("MCP '{}' not found", current_name));
        }
        if payload.name != current_name && config.tools.mcp_servers.contains_key(&payload.name) {
            return Err(anyhow!("MCP '{}' already exists", payload.name));
        }

        let was_disabled = config.tools.is_mcp_server_disabled(current_name);
        config.tools.mcp_servers.remove(current_name);
        config
            .tools
            .mcp_servers
            .insert(payload.name.clone(), Self::payload_to_config(&payload)?);
        if payload.name != current_name {
            config
                .tools
                .mcp_manager
                .disabled_servers
                .retain(|name| name != current_name);
            if was_disabled && payload.enabled {
                // no-op, handled below
            }
        }
        Self::set_enabled_flag(&mut config, &payload.name, payload.enabled);
        self.loader.save(&config)?;
        self.get_mcp(&payload.name)
    }

    pub fn delete_mcp(&self, name: &str) -> anyhow::Result<()> {
        let mut config = self.loader.load()?;
        let removed = config.tools.mcp_servers.remove(name);
        config
            .tools
            .mcp_manager
            .disabled_servers
            .retain(|item| item != name);
        if removed.is_none() {
            return Err(anyhow!("MCP '{}' not found", name));
        }
        self.loader.save(&config)?;
        Ok(())
    }

    pub fn set_enabled(&self, name: &str, enabled: bool) -> anyhow::Result<McpServerDto> {
        let mut config = self.loader.load()?;
        if !config.tools.mcp_servers.contains_key(name) {
            return Err(anyhow!("MCP '{}' not found", name));
        }
        Self::set_enabled_flag(&mut config, name, enabled);
        self.loader.save(&config)?;
        self.get_mcp(name)
    }

    pub fn get_mcp(&self, name: &str) -> anyhow::Result<McpServerDto> {
        let config = self.loader.load()?;
        let server = config
            .tools
            .mcp_servers
            .get(name)
            .ok_or_else(|| anyhow!("MCP '{}' not found", name))?;
        Ok(self.to_dto(&config, name, server))
    }

    pub fn active_servers(&self) -> anyhow::Result<HashMap<String, MCPServerConfig>> {
        let config = self.loader.load()?;
        Ok(config.tools.active_mcp_servers())
    }

    fn to_dto(&self, config: &Config, name: &str, server: &MCPServerConfig) -> McpServerDto {
        let enabled = !config.tools.is_mcp_server_disabled(name);
        let transport = if !server.command.trim().is_empty() {
            "stdio"
        } else if !server.url.trim().is_empty() {
            "http"
        } else {
            "invalid"
        };
        let status = if !enabled {
            McpConnectionStatusDto {
                state: "disabled".to_string(),
                connected: false,
                applied: false,
                tool_count: 0,
                error: None,
                checked_at: None,
            }
        } else {
            match probe_mcp_server(name, server) {
                Ok(tool_count) => McpConnectionStatusDto {
                    state: "connected".to_string(),
                    connected: true,
                    applied: true,
                    tool_count,
                    error: None,
                    checked_at: Some(Utc::now().to_rfc3339()),
                },
                Err(error) => McpConnectionStatusDto {
                    state: if transport == "invalid" {
                        "invalid".to_string()
                    } else {
                        "degraded".to_string()
                    },
                    connected: false,
                    applied: true,
                    tool_count: 0,
                    error: Some(error),
                    checked_at: Some(Utc::now().to_rfc3339()),
                },
            }
        };

        McpServerDto {
            name: name.to_string(),
            enabled,
            transport: transport.to_string(),
            command: server.command.clone(),
            args: server.args.clone(),
            env: server.env.clone(),
            url: server.url.clone(),
            tool_timeout: server.tool_timeout,
            status,
        }
    }

    fn payload_to_config(payload: &McpServerUpsert) -> anyhow::Result<MCPServerConfig> {
        let config = MCPServerConfig {
            command: payload.command.trim().to_string(),
            args: payload.args.clone(),
            env: payload.env.clone(),
            url: payload.url.trim().to_string(),
            tool_timeout: payload.tool_timeout.max(1),
        };
        Self::validate_server(&payload.name, &config)?;
        Ok(config)
    }

    fn validate_name(&self, name: &str) -> anyhow::Result<()> {
        if name.trim().is_empty() {
            return Err(anyhow!("MCP name is required"));
        }
        Ok(())
    }

    fn validate_server(name: &str, server: &MCPServerConfig) -> anyhow::Result<()> {
        let has_stdio = !server.command.trim().is_empty();
        let has_http = !server.url.trim().is_empty();
        if !has_stdio && !has_http {
            return Err(anyhow!(
                "tools.mcp_servers.{} must set either command (stdio) or url (http)",
                name
            ));
        }
        if has_stdio && has_http {
            return Err(anyhow!(
                "tools.mcp_servers.{} cannot set both command and url at the same time",
                name
            ));
        }
        Ok(())
    }

    fn set_enabled_flag(config: &mut Config, name: &str, enabled: bool) {
        config
            .tools
            .mcp_manager
            .disabled_servers
            .retain(|item| item != name);
        if !enabled {
            config
                .tools
                .mcp_manager
                .disabled_servers
                .push(name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_config(config_dir: &TempDir) -> ConfigLoader {
        let loader = ConfigLoader::with_dir(config_dir.path());
        let config = Config::default();
        loader.save(&config).unwrap();
        loader
    }

    #[test]
    fn create_and_disable_mcp_roundtrip() {
        let config_dir = TempDir::new().unwrap();
        let loader = write_config(&config_dir);
        let service = McpService::new(loader.clone());

        let dto = service
            .create_mcp(McpServerUpsert {
                name: "remote".to_string(),
                enabled: false,
                command: String::new(),
                args: Vec::new(),
                env: HashMap::new(),
                url: "http://127.0.0.1:9000/mcp".to_string(),
                tool_timeout: 30,
            })
            .unwrap();
        assert_eq!(dto.name, "remote");
        assert!(!dto.enabled);

        let config = loader.load().unwrap();
        assert!(config.tools.is_mcp_server_disabled("remote"));
    }

    #[test]
    fn delete_mcp_removes_disabled_flag() {
        let config_dir = TempDir::new().unwrap();
        let loader = write_config(&config_dir);
        let service = McpService::new(loader.clone());
        service
            .create_mcp(McpServerUpsert {
                name: "stdio".to_string(),
                enabled: false,
                command: "uvx".to_string(),
                args: vec!["mcp-server-filesystem".to_string()],
                env: HashMap::new(),
                url: String::new(),
                tool_timeout: 30,
            })
            .unwrap();

        service.delete_mcp("stdio").unwrap();
        let config = loader.load().unwrap();
        assert!(!config.tools.mcp_servers.contains_key("stdio"));
        assert!(!config.tools.is_mcp_server_disabled("stdio"));
        let _ = fs::remove_dir_all(config_dir.path());
    }
}
