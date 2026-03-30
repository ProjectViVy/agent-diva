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
    /// Soul and identity behavior
    #[serde(default)]
    pub soul: AgentSoulConfig,
}

/// Default agent settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    /// Workspace directory
    pub workspace: String,
    /// Explicit default provider selection
    #[serde(default)]
    pub provider: Option<String>,
    /// Default model
    pub model: String,
    /// Maximum tokens
    pub max_tokens: u32,
    /// Temperature
    pub temperature: f32,
    /// Maximum tool iterations
    pub max_tool_iterations: u32,
    /// Optional reasoning effort for thinking-capable models (low/medium/high)
    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            workspace: "~/.agent-diva/workspace".to_string(),
            provider: Some("deepseek".to_string()),
            model: "deepseek-chat".to_string(),
            max_tokens: 8192,
            temperature: 0.7,
            max_tool_iterations: 20,
            reasoning_effort: None,
        }
    }
}

/// Soul/identity settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSoulConfig {
    /// Whether soul context injection is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Max characters loaded from each soul markdown file.
    #[serde(default = "default_soul_max_chars")]
    pub max_chars: usize,
    /// Whether to notify user when soul files are updated.
    #[serde(default = "default_true")]
    pub notify_on_change: bool,
    /// If true, BOOTSTRAP.md is only used until bootstrap is completed.
    #[serde(default = "default_true")]
    pub bootstrap_once: bool,
    /// Rolling window in seconds for frequent soul-change hints.
    #[serde(default = "default_soul_window_secs")]
    pub frequent_change_window_secs: u64,
    /// Minimum soul-changing turns in window to trigger hints.
    #[serde(default = "default_soul_change_threshold")]
    pub frequent_change_threshold: usize,
    /// Add boundary confirmation hint when SOUL.md changes.
    #[serde(default = "default_true")]
    pub boundary_confirmation_hint: bool,
}

impl Default for AgentSoulConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_chars: 4000,
            notify_on_change: true,
            bootstrap_once: true,
            frequent_change_window_secs: 600,
            frequent_change_threshold: 3,
            boundary_confirmation_hint: true,
        }
    }
}

fn default_soul_max_chars() -> usize {
    4000
}

fn default_soul_window_secs() -> u64 {
    600
}

fn default_soul_change_threshold() -> usize {
    3
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
    #[serde(default)]
    pub matrix: MatrixConfig,
    #[serde(
        default,
        rename = "neuro-link",
        alias = "neuro_link",
        alias = "generic_pipe"
    )]
    pub neuro_link: NeuroLinkConfig,
    #[serde(default)]
    pub irc: IrcConfig,
    #[serde(default)]
    pub mattermost: MattermostConfig,
    #[serde(default)]
    pub nextcloud_talk: NextcloudTalkConfig,
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
    /// When set, only guild messages from this server are handled (DMs are still allowed).
    #[serde(default)]
    pub guild_id: Option<String>,
    /// In guild channels, require @mention of the bot (unless sender is in `group_reply_allowed_sender_ids`).
    #[serde(default)]
    pub mention_only: bool,
    /// When true, process messages from other bots.
    #[serde(default)]
    pub listen_to_bots: bool,
    /// User IDs that may trigger the bot in guild channels without @mention when `mention_only` is true.
    #[serde(default)]
    pub group_reply_allowed_sender_ids: Vec<String>,
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
            guild_id: None,
            mention_only: false,
            listen_to_bots: false,
            group_reply_allowed_sender_ids: Vec::new(),
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

/// Neuro-link (WebSocket server) channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuroLinkConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_pipe_host")]
    pub host: String,
    #[serde(default = "default_pipe_port")]
    pub port: u16,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_pipe_host() -> String {
    "0.0.0.0".to_string()
}
fn default_pipe_port() -> u16 {
    9100
}

impl Default for NeuroLinkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_pipe_host(),
            port: default_pipe_port(),
            allow_from: Vec::new(),
        }
    }
}

/// Matrix channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_matrix_homeserver")]
    pub homeserver: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default = "default_true")]
    pub e2ee_enabled: bool,
    #[serde(default = "default_matrix_media_limit")]
    pub max_media_bytes: usize,
    #[serde(default)]
    pub allow_from: Vec<String>,
    #[serde(default)]
    pub group_allow_from: Vec<String>,
    #[serde(default = "default_matrix_sync_timeout")]
    pub sync_timeout_ms: u64,
    #[serde(default = "default_matrix_sync_stop_grace")]
    pub sync_stop_grace_seconds: u64,
}

fn default_matrix_homeserver() -> String {
    "https://matrix.org".to_string()
}
fn default_matrix_media_limit() -> usize {
    20 * 1024 * 1024
}
fn default_matrix_sync_timeout() -> u64 {
    30_000
}
fn default_matrix_sync_stop_grace() -> u64 {
    8
}

impl Default for MatrixConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            homeserver: default_matrix_homeserver(),
            user_id: String::new(),
            access_token: String::new(),
            device_id: String::new(),
            e2ee_enabled: true,
            max_media_bytes: default_matrix_media_limit(),
            allow_from: Vec::new(),
            group_allow_from: Vec::new(),
            sync_timeout_ms: default_matrix_sync_timeout(),
            sync_stop_grace_seconds: default_matrix_sync_stop_grace(),
        }
    }
}

/// IRC channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrcConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub server: String,
    #[serde(default = "default_irc_port")]
    pub port: u16,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub server_password: Option<String>,
    #[serde(default)]
    pub nickserv_password: Option<String>,
    #[serde(default)]
    pub sasl_password: Option<String>,
    #[serde(default = "default_true")]
    pub use_tls: bool,
    #[serde(default = "default_true")]
    pub verify_tls: bool,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_irc_port() -> u16 {
    6697
}

impl Default for IrcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server: String::new(),
            port: default_irc_port(),
            nickname: String::new(),
            username: String::new(),
            channels: Vec::new(),
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            use_tls: true,
            verify_tls: true,
            allow_from: Vec::new(),
        }
    }
}

/// Mattermost channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattermostConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub channel_id: String,
    #[serde(default = "default_true")]
    pub thread_replies: bool,
    #[serde(default)]
    pub mention_only: bool,
    #[serde(default = "default_mm_poll_interval")]
    pub poll_interval_seconds: u64,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_mm_poll_interval() -> u64 {
    3
}

impl Default for MattermostConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            bot_token: String::new(),
            channel_id: String::new(),
            thread_replies: true,
            mention_only: false,
            poll_interval_seconds: default_mm_poll_interval(),
            allow_from: Vec::new(),
        }
    }
}

/// Nextcloud Talk channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextcloudTalkConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub app_token: String,
    #[serde(default)]
    pub room_token: String,
    #[serde(default = "default_nc_poll_interval")]
    pub poll_interval_seconds: u64,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_nc_poll_interval() -> u64 {
    5
}

impl Default for NextcloudTalkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            app_token: String::new(),
            room_token: String::new(),
            poll_interval_seconds: default_nc_poll_interval(),
            allow_from: Vec::new(),
        }
    }
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
    #[serde(default)]
    pub custom_providers: HashMap<String, CustomProviderConfig>,
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
    #[serde(default)]
    pub custom_models: Vec<String>,
}

/// User-defined provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomProviderConfig {
    #[serde(default)]
    pub display_name: String,
    #[serde(default = "default_custom_provider_api_type")]
    pub api_type: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub extra_headers: Option<HashMap<String, String>>,
}

fn default_custom_provider_api_type() -> String {
    "openai".to_string()
}

impl ProvidersConfig {
    pub const BUILTIN_PROVIDER_IDS: [&'static str; 13] = [
        "anthropic",
        "openai",
        "openrouter",
        "deepseek",
        "groq",
        "zhipu",
        "dashscope",
        "vllm",
        "gemini",
        "moonshot",
        "minimax",
        "aihubmix",
        "custom",
    ];

    pub fn builtin_provider_names() -> &'static [&'static str] {
        &Self::BUILTIN_PROVIDER_IDS
    }

    pub fn get(&self, name: &str) -> Option<&ProviderConfig> {
        match name {
            "anthropic" => Some(&self.anthropic),
            "openai" => Some(&self.openai),
            "openrouter" => Some(&self.openrouter),
            "deepseek" => Some(&self.deepseek),
            "groq" => Some(&self.groq),
            "zhipu" => Some(&self.zhipu),
            "dashscope" => Some(&self.dashscope),
            "vllm" => Some(&self.vllm),
            "gemini" => Some(&self.gemini),
            "moonshot" => Some(&self.moonshot),
            "minimax" => Some(&self.minimax),
            "aihubmix" => Some(&self.aihubmix),
            "custom" => Some(&self.custom),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ProviderConfig> {
        match name {
            "anthropic" => Some(&mut self.anthropic),
            "openai" => Some(&mut self.openai),
            "openrouter" => Some(&mut self.openrouter),
            "deepseek" => Some(&mut self.deepseek),
            "groq" => Some(&mut self.groq),
            "zhipu" => Some(&mut self.zhipu),
            "dashscope" => Some(&mut self.dashscope),
            "vllm" => Some(&mut self.vllm),
            "gemini" => Some(&mut self.gemini),
            "moonshot" => Some(&mut self.moonshot),
            "minimax" => Some(&mut self.minimax),
            "aihubmix" => Some(&mut self.aihubmix),
            "custom" => Some(&mut self.custom),
            _ => None,
        }
    }

    pub fn get_custom(&self, name: &str) -> Option<&CustomProviderConfig> {
        self.custom_providers.get(name)
    }

    pub fn get_custom_mut(&mut self, name: &str) -> Option<&mut CustomProviderConfig> {
        self.custom_providers.get_mut(name)
    }

    pub fn is_builtin_provider(name: &str) -> bool {
        Self::BUILTIN_PROVIDER_IDS.contains(&name)
    }
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
    #[serde(default, rename = "mcpManager", alias = "mcp_manager")]
    pub mcp_manager: MCPManagerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPManagerConfig {
    #[serde(default)]
    pub disabled_servers: Vec<String>,
}

impl ToolsConfig {
    pub fn active_mcp_servers(&self) -> HashMap<String, MCPServerConfig> {
        self.mcp_servers
            .iter()
            .filter(|(name, _)| !self.is_mcp_server_disabled(name))
            .map(|(name, cfg)| (name.clone(), cfg.clone()))
            .collect()
    }

    pub fn is_mcp_server_disabled(&self, name: &str) -> bool {
        self.mcp_manager
            .disabled_servers
            .iter()
            .any(|server| server == name)
    }
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
    /// Per-tool timeout in seconds (default: 30)
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout: u64,
}

fn default_tool_timeout() -> u64 {
    30
}

/// Web tools configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebToolsConfig {
    #[serde(default)]
    pub search: WebSearchConfig,
    #[serde(default)]
    pub fetch: WebFetchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Web search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default = "default_search_provider")]
    pub provider: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_search_provider() -> String {
    "bocha".to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_max_results() -> u32 {
    5
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            provider: default_search_provider(),
            enabled: default_enabled(),
            api_key: String::new(),
            max_results: default_max_results(),
        }
    }
}

impl Default for WebFetchConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
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
