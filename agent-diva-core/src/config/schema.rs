//! Configuration schema definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root configuration for agent-diva
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Agent configuration
    pub agents: AgentsConfig,
    /// Channel configuration
    pub channels: ChannelsConfig,
    /// Provider configuration
    pub providers: ProvidersConfig,
    /// Gateway configuration
    pub gateway: GatewayConfig,
    /// Tools configuration
    pub tools: ToolsConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Default log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log format (text, json)
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Directory for log files
    #[serde(default = "default_log_dir")]
    pub dir: String,
    /// Module-specific overrides
    #[serde(default)]
    pub overrides: HashMap<String, String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "text".to_string()
}

fn default_log_dir() -> String {
    "logs".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            dir: default_log_dir(),
            overrides: HashMap::new(),
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    /// Default agent settings
    pub defaults: AgentDefaults,
}

/// Default agent settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    /// Workspace directory
    pub workspace: String,
    /// Default model
    pub model: String,
    /// Maximum tokens
    pub max_tokens: u32,
    /// Temperature
    pub temperature: f32,
    /// Maximum tool iterations
    pub max_tool_iterations: u32,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            workspace: "~/.agent-diva/workspace".to_string(),
            model: "anthropic/claude-opus-4-5".to_string(),
            max_tokens: 8192,
            temperature: 0.7,
            max_tool_iterations: 20,
        }
    }
}

/// Channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub whatsapp: WhatsAppConfig,
    #[serde(default)]
    pub feishu: FeishuConfig,
    #[serde(default)]
    pub dingtalk: DingTalkConfig,
    #[serde(default)]
    pub email: EmailConfig,
    #[serde(default)]
    pub slack: SlackConfig,
    #[serde(default)]
    pub qq: QQConfig,
}

/// Telegram channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
    #[serde(default)]
    pub proxy: Option<String>,
}

/// Discord channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
    #[serde(default = "default_discord_gateway")]
    pub gateway_url: String,
    #[serde(default = "default_discord_intents")]
    pub intents: u64,
}

fn default_discord_gateway() -> String {
    "wss://gateway.discord.gg/?v=10&encoding=json".to_string()
}

fn default_discord_intents() -> u64 {
    37377 // GUILDS + GUILD_MESSAGES + DIRECT_MESSAGES + MESSAGE_CONTENT
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allow_from: Vec::new(),
            gateway_url: default_discord_gateway(),
            intents: default_discord_intents(),
        }
    }
}

/// WhatsApp channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_whatsapp_bridge")]
    pub bridge_url: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_whatsapp_bridge() -> String {
    "ws://localhost:3001".to_string()
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bridge_url: default_whatsapp_bridge(),
            allow_from: Vec::new(),
        }
    }
}

/// Feishu/Lark channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeishuConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_secret: String,
    #[serde(default)]
    pub encrypt_key: String,
    #[serde(default)]
    pub verification_token: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

/// DingTalk channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub robot_code: String,
    #[serde(default = "default_dingtalk_policy")]
    pub dm_policy: String,
    #[serde(default = "default_dingtalk_policy")]
    pub group_policy: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_dingtalk_policy() -> String {
    "open".to_string()
}

impl Default for DingTalkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            client_id: String::new(),
            client_secret: String::new(),
            robot_code: String::new(),
            dm_policy: default_dingtalk_policy(),
            group_policy: default_dingtalk_policy(),
            allow_from: Vec::new(),
        }
    }
}

/// Email channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub consent_granted: bool,
    // IMAP settings
    #[serde(default)]
    pub imap_host: String,
    #[serde(default = "default_imap_port")]
    pub imap_port: u16,
    #[serde(default)]
    pub imap_username: String,
    #[serde(default)]
    pub imap_password: String,
    #[serde(default = "default_imap_mailbox")]
    pub imap_mailbox: String,
    #[serde(default = "default_true")]
    pub imap_use_ssl: bool,
    // SMTP settings
    #[serde(default)]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default)]
    pub smtp_username: String,
    #[serde(default)]
    pub smtp_password: String,
    #[serde(default = "default_true")]
    pub smtp_use_tls: bool,
    #[serde(default)]
    pub smtp_use_ssl: bool,
    #[serde(default)]
    pub from_address: String,
    // Behavior
    #[serde(default = "default_true")]
    pub auto_reply_enabled: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
    #[serde(default = "default_true")]
    pub mark_seen: bool,
    #[serde(default = "default_max_body")]
    pub max_body_chars: usize,
    #[serde(default = "default_subject_prefix")]
    pub subject_prefix: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
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
fn default_true() -> bool {
    true
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            consent_granted: false,
            imap_host: String::new(),
            imap_port: default_imap_port(),
            imap_username: String::new(),
            imap_password: String::new(),
            imap_mailbox: default_imap_mailbox(),
            imap_use_ssl: true,
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_use_tls: true,
            smtp_use_ssl: false,
            from_address: String::new(),
            auto_reply_enabled: true,
            poll_interval_seconds: default_poll_interval(),
            mark_seen: true,
            max_body_chars: default_max_body(),
            subject_prefix: default_subject_prefix(),
            allow_from: Vec::new(),
        }
    }
}

/// Slack channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_slack_mode")]
    pub mode: String,
    #[serde(default)]
    pub webhook_path: String,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub app_token: String,
    #[serde(default = "default_true")]
    pub user_token_read_only: bool,
    #[serde(default = "default_slack_policy")]
    pub group_policy: String,
    #[serde(default)]
    pub group_allow_from: Vec<String>,
    #[serde(default)]
    pub dm: SlackDMConfig,
}

fn default_slack_mode() -> String {
    "socket".to_string()
}
fn default_slack_policy() -> String {
    "mention".to_string()
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_slack_mode(),
            webhook_path: "/slack/events".to_string(),
            bot_token: String::new(),
            app_token: String::new(),
            user_token_read_only: true,
            group_policy: default_slack_policy(),
            group_allow_from: Vec::new(),
            dm: SlackDMConfig::default(),
        }
    }
}

/// Slack DM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackDMConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_slack_dm_policy")]
    pub policy: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_slack_dm_policy() -> String {
    "open".to_string()
}

impl Default for SlackDMConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy: default_slack_dm_policy(),
            allow_from: Vec::new(),
        }
    }
}

/// QQ channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QQConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub secret: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub anthropic: ProviderConfig,
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub openrouter: ProviderConfig,
    #[serde(default)]
    pub deepseek: ProviderConfig,
    #[serde(default)]
    pub groq: ProviderConfig,
    #[serde(default)]
    pub zhipu: ProviderConfig,
    #[serde(default)]
    pub dashscope: ProviderConfig,
    #[serde(default)]
    pub vllm: ProviderConfig,
    #[serde(default)]
    pub gemini: ProviderConfig,
    #[serde(default)]
    pub moonshot: ProviderConfig,
    #[serde(default)]
    pub minimax: ProviderConfig,
    #[serde(default)]
    pub aihubmix: ProviderConfig,
    #[serde(default)]
    pub custom: ProviderConfig,
}

/// Individual provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(default)]
    pub extra_headers: Option<HashMap<String, String>>,
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    18790
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub web: WebToolsConfig,
    #[serde(default)]
    pub exec: ExecToolConfig,
    #[serde(default)]
    pub restrict_to_workspace: bool,
    #[serde(default, rename = "mcpServers", alias = "mcp_servers")]
    pub mcp_servers: HashMap<String, MCPServerConfig>,
}

/// MCP server connection configuration (stdio or HTTP)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPServerConfig {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub url: String,
}

/// Web tools configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebToolsConfig {
    #[serde(default)]
    pub search: WebSearchConfig,
}

/// Web search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_max_results() -> u32 {
    5
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            max_results: default_max_results(),
        }
    }
}

/// Exec tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecToolConfig {
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    60
}

impl Default for ExecToolConfig {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
        }
    }
}
