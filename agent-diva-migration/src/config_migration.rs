//! Configuration migration from Python to Rust format
//!
//! Python version uses JSON with camelCase keys
//! Rust version uses JSON (compatible format)

use anyhow::{Context, Result};
use agent_diva_core::config::schema::*;
use agent_diva_core::config::Config;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Result of configuration migration
#[derive(Debug)]
pub struct MigrationResult {
    pub migrated: bool,
    pub already_exists: bool,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
}

/// Python version configuration structure (camelCase)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PythonConfig {
    #[serde(default)]
    agents: PythonAgentsConfig,
    #[serde(default)]
    channels: PythonChannelsConfig,
    #[serde(default)]
    providers: PythonProvidersConfig,
    #[serde(default)]
    gateway: PythonGatewayConfig,
    #[serde(default)]
    tools: PythonToolsConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonAgentsConfig {
    #[serde(default)]
    defaults: PythonAgentDefaults,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonAgentDefaults {
    #[serde(default = "default_workspace")]
    workspace: String,
    #[serde(default = "default_model")]
    model: String,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
    #[serde(default = "default_temperature")]
    temperature: f32,
    #[serde(default = "default_max_tool_iterations")]
    max_tool_iterations: u32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonChannelsConfig {
    #[serde(default)]
    telegram: PythonTelegramConfig,
    #[serde(default)]
    discord: PythonDiscordConfig,
    #[serde(default)]
    whatsapp: PythonWhatsAppConfig,
    #[serde(default)]
    feishu: PythonFeishuConfig,
    #[serde(default)]
    dingtalk: PythonDingTalkConfig,
    #[serde(default)]
    email: PythonEmailConfig,
    #[serde(default)]
    slack: PythonSlackConfig,
    #[serde(default)]
    qq: PythonQQConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonTelegramConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    token: String,
    #[serde(default)]
    allow_from: Vec<String>,
    #[serde(default)]
    proxy: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonDiscordConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    token: String,
    #[serde(default)]
    allow_from: Vec<String>,
    #[serde(default = "default_discord_gateway")]
    gateway_url: String,
    #[serde(default = "default_discord_intents")]
    intents: u64,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonWhatsAppConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_whatsapp_bridge")]
    bridge_url: String,
    #[serde(default)]
    allow_from: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonFeishuConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    app_id: String,
    #[serde(default)]
    app_secret: String,
    #[serde(default)]
    encrypt_key: String,
    #[serde(default)]
    verification_token: String,
    #[serde(default)]
    allow_from: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonDingTalkConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    client_secret: String,
    #[serde(default)]
    allow_from: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonEmailConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    consent_granted: bool,
    #[serde(default)]
    imap_host: String,
    #[serde(default = "default_imap_port")]
    imap_port: u16,
    #[serde(default)]
    imap_username: String,
    #[serde(default)]
    imap_password: String,
    #[serde(default = "default_imap_mailbox")]
    imap_mailbox: String,
    #[serde(default = "default_true")]
    imap_use_ssl: bool,
    #[serde(default)]
    smtp_host: String,
    #[serde(default = "default_smtp_port")]
    smtp_port: u16,
    #[serde(default)]
    smtp_username: String,
    #[serde(default)]
    smtp_password: String,
    #[serde(default = "default_true")]
    smtp_use_tls: bool,
    #[serde(default)]
    smtp_use_ssl: bool,
    #[serde(default)]
    from_address: String,
    #[serde(default = "default_true")]
    auto_reply_enabled: bool,
    #[serde(default = "default_poll_interval")]
    poll_interval_seconds: u64,
    #[serde(default = "default_true")]
    mark_seen: bool,
    #[serde(default = "default_max_body")]
    max_body_chars: usize,
    #[serde(default = "default_subject_prefix")]
    subject_prefix: String,
    #[serde(default)]
    allow_from: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonSlackConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_slack_mode")]
    mode: String,
    #[serde(default)]
    webhook_path: String,
    #[serde(default)]
    bot_token: String,
    #[serde(default)]
    app_token: String,
    #[serde(default = "default_true")]
    user_token_read_only: bool,
    #[serde(default = "default_slack_policy")]
    group_policy: String,
    #[serde(default)]
    group_allow_from: Vec<String>,
    #[serde(default)]
    dm: PythonSlackDMConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonSlackDMConfig {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_slack_dm_policy")]
    policy: String,
    #[serde(default)]
    allow_from: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonQQConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    app_id: String,
    #[serde(default)]
    secret: String,
    #[serde(default)]
    allow_from: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonProvidersConfig {
    #[serde(default)]
    anthropic: PythonProviderConfig,
    #[serde(default)]
    openai: PythonProviderConfig,
    #[serde(default)]
    openrouter: PythonProviderConfig,
    #[serde(default)]
    deepseek: PythonProviderConfig,
    #[serde(default)]
    groq: PythonProviderConfig,
    #[serde(default)]
    zhipu: PythonProviderConfig,
    #[serde(default)]
    dashscope: PythonProviderConfig,
    #[serde(default)]
    vllm: PythonProviderConfig,
    #[serde(default)]
    gemini: PythonProviderConfig,
    #[serde(default)]
    moonshot: PythonProviderConfig,
    #[serde(default)]
    minimax: PythonProviderConfig,
    #[serde(default)]
    aihubmix: PythonProviderConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonProviderConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    api_base: Option<String>,
    #[serde(default)]
    extra_headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonGatewayConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonToolsConfig {
    #[serde(default)]
    web: PythonWebToolsConfig,
    #[serde(default)]
    exec: PythonExecToolConfig,
    #[serde(default)]
    restrict_to_workspace: bool,
    #[serde(default)]
    mcp_servers: HashMap<String, PythonMCPServerConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonWebToolsConfig {
    #[serde(default)]
    search: PythonWebSearchConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonWebSearchConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_max_results")]
    max_results: u32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonExecToolConfig {
    #[serde(default = "default_timeout")]
    timeout: u64,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PythonMCPServerConfig {
    #[serde(default)]
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    url: String,
}

// Default value functions
fn default_workspace() -> String {
    "~/.agent-diva/workspace".to_string()
}

fn default_model() -> String {
    "anthropic/claude-opus-4-5".to_string()
}

fn default_max_tokens() -> u32 {
    8192
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tool_iterations() -> u32 {
    20
}

fn default_discord_gateway() -> String {
    "wss://gateway.discord.gg/?v=10&encoding=json".to_string()
}

fn default_discord_intents() -> u64 {
    37377
}

fn default_whatsapp_bridge() -> String {
    "ws://localhost:3001".to_string()
}

fn default_imap_port() -> u16 {
    993
}

fn default_imap_mailbox() -> String {
    "INBOX".to_string()
}

fn default_smtp_port() -> u16 {
    587
}

fn default_poll_interval() -> u64 {
    30
}

fn default_max_body() -> usize {
    12000
}

fn default_subject_prefix() -> String {
    "Re: ".to_string()
}

fn default_slack_mode() -> String {
    "socket".to_string()
}

fn default_slack_policy() -> String {
    "mention".to_string()
}

fn default_slack_dm_policy() -> String {
    "open".to_string()
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    18790
}

fn default_max_results() -> u32 {
    5
}

fn default_timeout() -> u64 {
    60
}

fn default_true() -> bool {
    true
}

/// Configuration migrator
pub struct ConfigMigrator {
    source_dir: PathBuf,
    target_dir: PathBuf,
}

impl ConfigMigrator {
    /// Create a new config migrator
    pub fn new(source_dir: impl AsRef<Path>, target_dir: impl AsRef<Path>) -> Self {
        Self {
            source_dir: source_dir.as_ref().to_path_buf(),
            target_dir: target_dir.as_ref().to_path_buf(),
        }
    }

    /// Migrate configuration from Python to Rust format
    pub async fn migrate(&self, dry_run: bool) -> Result<MigrationResult> {
        let source_path = self.source_dir.join("config.json");
        let target_path = self.target_dir.join("config.json");

        // Check if source exists
        if !source_path.exists() {
            return Ok(MigrationResult {
                migrated: false,
                already_exists: false,
                source_path,
                target_path,
            });
        }

        // Check if target already exists
        if target_path.exists() {
            warn!("Config already exists at target, skipping migration");
            return Ok(MigrationResult {
                migrated: false,
                already_exists: true,
                source_path,
                target_path,
            });
        }

        // Read and parse Python config
        let content = tokio::fs::read_to_string(&source_path)
            .await
            .with_context(|| format!("Failed to read config from {}", source_path.display()))?;

        let python_config: PythonConfig =
            serde_json::from_str(&content).with_context(|| "Failed to parse Python config")?;

        // Convert to Rust config
        let rust_config = self.convert_config(python_config);

        if dry_run {
            info!(
                "Dry run: would migrate config from {:?} to {:?}",
                source_path, target_path
            );
            return Ok(MigrationResult {
                migrated: true,
                already_exists: false,
                source_path,
                target_path,
            });
        }

        // Save Rust config
        tokio::fs::create_dir_all(&self.target_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create target directory {}",
                    self.target_dir.display()
                )
            })?;

        let config_json =
            serde_json::to_string_pretty(&rust_config).context("Failed to serialize config")?;

        tokio::fs::write(&target_path, config_json)
            .await
            .with_context(|| format!("Failed to write config to {}", target_path.display()))?;

        info!("Config migrated successfully to {:?}", target_path);

        Ok(MigrationResult {
            migrated: true,
            already_exists: false,
            source_path,
            target_path,
        })
    }

    /// Convert Python config to Rust config
    fn convert_config(&self, py: PythonConfig) -> Config {
        Config {
            agents: AgentsConfig {
                defaults: AgentDefaults {
                    workspace: py.agents.defaults.workspace,
                    model: py.agents.defaults.model,
                    max_tokens: py.agents.defaults.max_tokens,
                    temperature: py.agents.defaults.temperature,
                    max_tool_iterations: py.agents.defaults.max_tool_iterations,
                },
            },
            channels: ChannelsConfig {
                telegram: TelegramConfig {
                    enabled: py.channels.telegram.enabled,
                    token: py.channels.telegram.token,
                    allow_from: py.channels.telegram.allow_from,
                    proxy: py.channels.telegram.proxy,
                },
                discord: DiscordConfig {
                    enabled: py.channels.discord.enabled,
                    token: py.channels.discord.token,
                    allow_from: py.channels.discord.allow_from,
                    gateway_url: py.channels.discord.gateway_url,
                    intents: py.channels.discord.intents,
                },
                whatsapp: WhatsAppConfig {
                    enabled: py.channels.whatsapp.enabled,
                    bridge_url: py.channels.whatsapp.bridge_url,
                    allow_from: py.channels.whatsapp.allow_from,
                },
                feishu: FeishuConfig {
                    enabled: py.channels.feishu.enabled,
                    app_id: py.channels.feishu.app_id,
                    app_secret: py.channels.feishu.app_secret,
                    encrypt_key: py.channels.feishu.encrypt_key,
                    verification_token: py.channels.feishu.verification_token,
                    allow_from: py.channels.feishu.allow_from,
                },
                dingtalk: DingTalkConfig {
                    enabled: py.channels.dingtalk.enabled,
                    client_id: py.channels.dingtalk.client_id,
                    client_secret: py.channels.dingtalk.client_secret,
                    allow_from: py.channels.dingtalk.allow_from,
                },
                email: EmailConfig {
                    enabled: py.channels.email.enabled,
                    consent_granted: py.channels.email.consent_granted,
                    imap_host: py.channels.email.imap_host,
                    imap_port: py.channels.email.imap_port,
                    imap_username: py.channels.email.imap_username,
                    imap_password: py.channels.email.imap_password,
                    imap_mailbox: py.channels.email.imap_mailbox,
                    imap_use_ssl: py.channels.email.imap_use_ssl,
                    smtp_host: py.channels.email.smtp_host,
                    smtp_port: py.channels.email.smtp_port,
                    smtp_username: py.channels.email.smtp_username,
                    smtp_password: py.channels.email.smtp_password,
                    smtp_use_tls: py.channels.email.smtp_use_tls,
                    smtp_use_ssl: py.channels.email.smtp_use_ssl,
                    from_address: py.channels.email.from_address,
                    auto_reply_enabled: py.channels.email.auto_reply_enabled,
                    poll_interval_seconds: py.channels.email.poll_interval_seconds,
                    mark_seen: py.channels.email.mark_seen,
                    max_body_chars: py.channels.email.max_body_chars,
                    subject_prefix: py.channels.email.subject_prefix,
                    allow_from: py.channels.email.allow_from,
                },
                slack: SlackConfig {
                    enabled: py.channels.slack.enabled,
                    mode: py.channels.slack.mode,
                    webhook_path: py.channels.slack.webhook_path,
                    bot_token: py.channels.slack.bot_token,
                    app_token: py.channels.slack.app_token,
                    user_token_read_only: py.channels.slack.user_token_read_only,
                    group_policy: py.channels.slack.group_policy,
                    group_allow_from: py.channels.slack.group_allow_from,
                    dm: SlackDMConfig {
                        enabled: py.channels.slack.dm.enabled,
                        policy: py.channels.slack.dm.policy,
                        allow_from: py.channels.slack.dm.allow_from,
                    },
                },
                qq: QQConfig {
                    enabled: py.channels.qq.enabled,
                    app_id: py.channels.qq.app_id,
                    secret: py.channels.qq.secret,
                    allow_from: py.channels.qq.allow_from,
                },
            },
            providers: ProvidersConfig {
                anthropic: self.convert_provider(&py.providers.anthropic),
                openai: self.convert_provider(&py.providers.openai),
                openrouter: self.convert_provider(&py.providers.openrouter),
                deepseek: self.convert_provider(&py.providers.deepseek),
                groq: self.convert_provider(&py.providers.groq),
                zhipu: self.convert_provider(&py.providers.zhipu),
                dashscope: self.convert_provider(&py.providers.dashscope),
                vllm: self.convert_provider(&py.providers.vllm),
                gemini: self.convert_provider(&py.providers.gemini),
                moonshot: self.convert_provider(&py.providers.moonshot),
                minimax: self.convert_provider(&py.providers.minimax),
                aihubmix: self.convert_provider(&py.providers.aihubmix),
                custom: ProviderConfig::default(),
            },
            gateway: GatewayConfig {
                host: py.gateway.host,
                port: py.gateway.port,
            },
            tools: ToolsConfig {
                web: WebToolsConfig {
                    search: WebSearchConfig {
                        api_key: py.tools.web.search.api_key,
                        max_results: py.tools.web.search.max_results,
                    },
                },
                exec: ExecToolConfig {
                    timeout: py.tools.exec.timeout,
                },
                restrict_to_workspace: py.tools.restrict_to_workspace,
                mcp_servers: py
                    .tools
                    .mcp_servers
                    .into_iter()
                    .map(|(name, server)| {
                        (
                            name,
                            MCPServerConfig {
                                command: server.command,
                                args: server.args,
                                env: server.env,
                                url: server.url,
                            },
                        )
                    })
                    .collect(),
            },
        }
    }

    fn convert_provider(&self, py: &PythonProviderConfig) -> ProviderConfig {
        ProviderConfig {
            api_key: py.api_key.clone(),
            api_base: py.api_base.clone(),
            extra_headers: py.extra_headers.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::ConfigLoader;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_migration() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        tokio::fs::create_dir_all(&source_dir).await.unwrap();

        // Create a sample Python config
        let python_config = r#"{
            "agents": {
                "defaults": {
                    "workspace": "~/test",
                    "model": "gpt-4",
                    "maxTokens": 4096,
                    "temperature": 0.5,
                    "maxToolIterations": 10
                }
            },
            "channels": {
                "telegram": {
                    "enabled": true,
                    "token": "test-token",
                    "allowFrom": ["user1"]
                }
            },
            "providers": {
                "openai": {
                    "apiKey": "sk-test"
                }
            }
        }"#;

        tokio::fs::write(source_dir.join("config.json"), python_config)
            .await
            .unwrap();

        let migrator = ConfigMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert!(result.migrated);
        assert!(!result.already_exists);

        // Verify the migrated config
        let config_loader = ConfigLoader::with_dir(&target_dir);
        let config = config_loader.load().unwrap();

        assert_eq!(config.agents.defaults.model, "gpt-4");
        assert_eq!(config.agents.defaults.max_tokens, 4096);
        assert_eq!(config.channels.telegram.enabled, true);
        assert_eq!(config.channels.telegram.token, "test-token");
        assert_eq!(config.providers.openai.api_key, "sk-test");
    }

    #[tokio::test]
    async fn test_config_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        tokio::fs::create_dir_all(&source_dir).await.unwrap();
        tokio::fs::create_dir_all(&target_dir).await.unwrap();

        // Create source config
        tokio::fs::write(source_dir.join("config.json"), "{}")
            .await
            .unwrap();

        // Create target config
        tokio::fs::write(target_dir.join("config.json"), "{}")
            .await
            .unwrap();

        let migrator = ConfigMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert!(!result.migrated);
        assert!(result.already_exists);
    }

    #[tokio::test]
    async fn test_no_source_config() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        tokio::fs::create_dir_all(&source_dir).await.unwrap();

        let migrator = ConfigMigrator::new(&source_dir, &target_dir);
        let result = migrator.migrate(false).await.unwrap();

        assert!(!result.migrated);
        assert!(!result.already_exists);
    }
}
