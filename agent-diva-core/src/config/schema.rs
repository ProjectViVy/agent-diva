//! Configuration schema definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Sandbox configuration types (used by agent-diva-sandbox) ────────────────

/// Sandbox execution mode
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    /// No sandbox isolation — full host access
    DangerFullAccess,
    /// Read-only filesystem access
    ReadOnly,
    /// Write access limited to the workspace directory
    #[default]
    WorkspaceWrite,
}

/// When to ask the user for approval before executing a command
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AskForApproval {
    /// Never ask — auto-approve everything
    Never,
    /// Ask only when the command fails
    OnFailure,
    /// Ask for every command
    OnRequest,
    /// Ask unless the command is in a trusted list
    #[default]
    UnlessTrusted,
}

/// Windows-specific sandbox isolation level
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowsSandboxLevel {
    /// No Windows sandbox — rely on general sandbox mode only
    Disabled,
    /// Run with a restricted token (reduced privileges)
    #[default]
    RestrictedToken,
    /// Run with elevated isolation (AppContainer-like)
    Elevated,
}

/// Sandbox section in the root configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Sandbox execution mode
    #[serde(default)]
    pub mode: SandboxMode,
    /// Windows-specific sandbox level
    #[serde(default)]
    pub windows_level: WindowsSandboxLevel,
    /// Whether network access is allowed inside the sandbox
    #[serde(default)]
    pub network_access: bool,
    /// When to ask for user approval
    #[serde(default)]
    pub approval_policy: AskForApproval,
    /// Extra writable root paths (strings; resolved at runtime)
    #[serde(default)]
    pub writable_roots: Vec<String>,
    /// Glob patterns for paths that must never be written to
    #[serde(default)]
    pub protected_paths: Vec<String>,
    /// Patterns for commands that should be denied
    #[serde(default)]
    pub deny_patterns: Vec<String>,
    /// Default command timeout in seconds
    #[serde(default = "default_sandbox_timeout")]
    pub timeout_seconds: u64,
}

fn default_sandbox_timeout() -> u64 {
    60
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            mode: SandboxMode::default(),
            windows_level: WindowsSandboxLevel::default(),
            network_access: false,
            approval_policy: AskForApproval::default(),
            writable_roots: Vec::new(),
            protected_paths: Vec::new(),
            deny_patterns: Vec::new(),
            timeout_seconds: 60,
        }
    }
}

// ── Mask configuration types ───────────────────────────────────────────────

/// Tool access limits for a mask
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ToolLimits {
    /// Tools explicitly allowed (empty = all allowed)
    #[serde(default)]
    pub allow: Vec<String>,
    /// Tools explicitly denied
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Default settings for subagents spawned under a mask
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SubagentDefaults {
    /// Model override for subagents
    #[serde(default)]
    pub model: Option<String>,
    /// Maximum iteration count for subagents
    #[serde(default)]
    pub max_iterations: Option<u32>,
}

/// Agent operating mode for a mask.
///
/// Controls the behavioral envelope of the agent — whether it operates
/// normally or is restricted to a read-only reviewer posture.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Default mode — full read/write tool access.
    #[default]
    Normal,
    /// Reviewer mode — read-only tools only; write tools are excluded.
    Assist,
}

/// Configuration for a single mask (loaded from a .md file with YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MaskConfig {
    /// Optional stable identifier used for file naming and unambiguous switching.
    #[serde(default)]
    pub id: Option<String>,
    /// Display name (required)
    pub name: String,
    /// Emoji icon
    #[serde(default)]
    pub icon: Option<String>,
    /// Short description
    #[serde(default)]
    pub description: Option<String>,
    /// Agent operating mode (Normal or Assist/reviewer)
    #[serde(default)]
    pub mode: Option<AgentMode>,
    /// Model override when this mask is active
    #[serde(default)]
    pub model: Option<String>,
    /// Default settings for subagents
    #[serde(default)]
    pub subagent_defaults: SubagentDefaults,
    /// Tool access limits
    #[serde(default)]
    pub tool_limits: ToolLimits,
}

impl MaskConfig {
    /// Return the explicit `id` if present, otherwise a URL-safe slug derived from `name`.
    ///
    /// For names containing only non-ASCII characters, the slug is a deterministic
    /// short hash so that different names do not collapse to the same value.
    pub fn id_or_slug(&self) -> String {
        self.id
            .clone()
            .unwrap_or_else(|| slug_from_name(&self.name))
    }
}

/// Build a URL-safe slug from a mask display name.
fn slug_from_name(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        format!("h{:016x}", deterministic_hash(name))
    } else {
        slug
    }
}

fn deterministic_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// ── Subagent batch-spawn contracts ────────────────────────────────────────

/// Token usage statistics from an LLM call
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    /// Tokens consumed in the prompt
    #[serde(default)]
    pub prompt_tokens: u32,
    /// Tokens generated in the completion
    #[serde(default)]
    pub completion_tokens: u32,
    /// Total tokens used
    #[serde(default)]
    pub total_tokens: u32,
}

/// Terminal status of a subagent task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentStatus {
    /// Task completed successfully
    Ok,
    /// Task failed with an error
    Error,
    /// Task exceeded its time budget
    Timeout,
    /// Task was cancelled externally
    Cancelled,
}

/// Result produced by a single subagent after it finishes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubAgentResult {
    /// Correlates back to the originating SubAgentTask.id
    pub task_id: String,
    /// Terminal status
    pub status: SubAgentStatus,
    /// Human-readable summary of what the subagent did (LLM-generated)
    #[serde(default)]
    pub summary: Option<String>,
    /// Wall-clock time in milliseconds
    pub elapsed_ms: u64,
    /// Number of tool calls the subagent made
    pub tool_call_count: u32,
    /// Token usage breakdown (may be absent if the provider doesn't report it)
    #[serde(default)]
    pub token_usage: Option<TokenUsage>,
    /// Ordered list of tool names that were invoked during execution
    #[serde(default)]
    pub tool_trace: Option<Vec<String>>,
}

/// A single task inside a batch spawn request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubAgentTask {
    /// Caller-assigned unique identifier for this task
    pub id: String,
    /// High-level goal the subagent should accomplish
    pub goal: String,
    /// Optional additional context (e.g. prior conversation, file contents)
    #[serde(default)]
    pub context: Option<String>,
}

/// Request to spawn multiple subagents in a single call
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchSpawnRequest {
    /// The tasks to execute concurrently
    pub tasks: Vec<SubAgentTask>,
    /// Optional maximum iterations per subagent (defaults via resolve_max_iterations)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
}

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
    /// Memory authority and cutover configuration.
    #[serde(default)]
    pub memory: MemoryConfig,
    /// Self-evolution and AutoDream governance policy.
    #[serde(default)]
    pub self_evolution: SelfEvolutionConfig,
    /// Notebook / rhythm report generation settings.
    #[serde(default)]
    pub reports: ReportsConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Sandbox configuration
    #[serde(default)]
    pub sandbox: SandboxConfig,
    /// Mate (desktop avatar) configuration
    #[serde(default, rename = "mate", alias = "pet")]
    pub mate: MateConfig,
}

/// Memory authority configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryConfig {
    /// L1 startup index budget: maximum index lines injected into the system
    /// prompt. Full entries are never injected; retrieval goes through
    /// `memory_search` / `memory_list` (B2/B10 minimal-pointer principle).
    #[serde(default = "default_l1_index_lines")]
    pub l1_index_lines: usize,
}

fn default_l1_index_lines() -> usize {
    30
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            l1_index_lines: default_l1_index_lines(),
        }
    }
}

#[cfg(test)]
mod memory_authority_tests {
    use super::*;

    #[test]
    fn missing_memory_config_defaults_to_typed() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value.as_object_mut().unwrap().remove("memory");
        let config: Config = serde_json::from_value(value).unwrap();
        assert_eq!(config.memory.l1_index_lines, 30);
    }

    #[test]
    fn memory_section_without_authority_mode_defaults_to_typed() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("memory".to_string(), serde_json::json!({}));
        let config: Config = serde_json::from_value(value).unwrap();
        assert_eq!(config.memory.l1_index_lines, 30);
    }

    #[test]
    fn legacy_authority_mode_key_no_longer_selects_a_provider() {
        // The key is unstructured noise after the clean break: parsing still
        // succeeds and no authority-mode field exists to honor it.
        let config: MemoryConfig =
            serde_json::from_value(serde_json::json!({"authority_mode": "legacy"})).unwrap();
        assert_eq!(config.l1_index_lines, 30);
    }

    #[test]
    fn l1_index_lines_defaults_to_30() {
        let config: MemoryConfig = MemoryConfig::default();
        assert_eq!(config.l1_index_lines, 30);
        assert_eq!(MemoryConfig::default().l1_index_lines, 30);
    }

    #[test]
    fn l1_index_lines_explicit_value_is_honored() {
        let config: MemoryConfig =
            serde_json::from_value(serde_json::json!({"l1_index_lines": 5})).unwrap();
        assert_eq!(config.l1_index_lines, 5);
    }
}

/// Top-level report generation configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportsConfig {
    #[serde(default)]
    pub llm_curation: LlmCurationConfig,
}

/// LLM curation settings for manual/scheduled rhythm reports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmCurationConfig {
    /// Defaults to enabled; set false to always use deterministic fallback.
    #[serde(default)]
    pub enabled: bool,
    /// Optional provider override; inherits `agents.defaults.provider` when null.
    #[serde(default)]
    pub provider: Option<String>,
    /// Optional model override; inherits `agents.defaults.model` when null.
    /// Must be the raw model id for native endpoints (no gateway prefix rewrite).
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_report_language")]
    pub language: String,
    #[serde(default = "default_report_max_input_tokens")]
    pub max_input_tokens: u32,
    #[serde(default = "default_report_max_output_tokens")]
    pub max_output_tokens: u32,
    #[serde(default = "default_report_timeout_secs")]
    pub timeout_secs: u64,
    /// Currently only `deterministic` is supported.
    #[serde(default = "default_report_fallback")]
    pub fallback: String,
}

fn default_report_language() -> String {
    "zh-CN".to_string()
}

fn default_report_max_input_tokens() -> u32 {
    8000
}

fn default_report_max_output_tokens() -> u32 {
    1500
}

fn default_report_timeout_secs() -> u64 {
    60
}

fn default_report_fallback() -> String {
    "deterministic".to_string()
}

impl Default for LlmCurationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: None,
            model: None,
            language: default_report_language(),
            max_input_tokens: default_report_max_input_tokens(),
            max_output_tokens: default_report_max_output_tokens(),
            timeout_secs: default_report_timeout_secs(),
            fallback: default_report_fallback(),
        }
    }
}

impl LlmCurationConfig {
    /// Validate curation settings; returns human-readable errors.
    pub fn validate(&self) -> Result<(), String> {
        if self.timeout_secs == 0 {
            return Err("reports.llm_curation.timeout_secs must be > 0".to_string());
        }
        if self.max_input_tokens == 0 {
            return Err("reports.llm_curation.max_input_tokens must be > 0".to_string());
        }
        if self.max_output_tokens == 0 {
            return Err("reports.llm_curation.max_output_tokens must be > 0".to_string());
        }
        if self.fallback != "deterministic" {
            return Err("reports.llm_curation.fallback must be \"deterministic\"".to_string());
        }
        Ok(())
    }
}

/// Self-evolution policy configuration used by Evolution and AutoDream UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelfEvolutionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_autodream_frequency")]
    pub autodream_frequency: String,
    #[serde(default = "default_trigger_threshold_sessions")]
    pub trigger_threshold_sessions: u32,
    #[serde(default = "default_trigger_threshold_messages")]
    pub trigger_threshold_messages: u32,
    #[serde(default = "default_auto_merge_confidence")]
    pub auto_merge_confidence: f32,
    #[serde(default)]
    pub require_confirmation_for: Vec<String>,
}

fn default_autodream_frequency() -> String {
    "manual".to_string()
}

fn default_trigger_threshold_sessions() -> u32 {
    10
}

fn default_trigger_threshold_messages() -> u32 {
    50
}

fn default_auto_merge_confidence() -> f32 {
    0.95
}

impl Default for SelfEvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            autodream_frequency: default_autodream_frequency(),
            trigger_threshold_sessions: default_trigger_threshold_sessions(),
            trigger_threshold_messages: default_trigger_threshold_messages(),
            auto_merge_confidence: default_auto_merge_confidence(),
            require_confirmation_for: Vec::new(),
        }
    }
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
    /// Number of days to retain log files; 0 = keep all logs
    #[serde(default = "default_retention_days")]
    pub retention_days: u64,
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

fn default_retention_days() -> u64 {
    30
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            dir: default_log_dir(),
            overrides: HashMap::new(),
            retention_days: default_retention_days(),
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
    /// Optional thinking mode override (auto/on/off)
    #[serde(default)]
    pub thinking_mode: Option<crate::reasoning::ThinkingMode>,
    /// Per-session bounded turn admission limits.
    #[serde(default)]
    pub session_admission: SessionAdmissionConfig,
}

/// Additive configuration for serial admission within one canonical session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SessionAdmissionConfig {
    /// Maximum accepted waiters. The running turn is not counted.
    pub max_queue_depth: usize,
    /// Maximum queue wait, in seconds.
    pub wait_timeout: u64,
    /// Fully-idle slot retention, in seconds.
    pub idle_ttl: u64,
}

impl Default for SessionAdmissionConfig {
    fn default() -> Self {
        Self {
            max_queue_depth: 2,
            wait_timeout: 30,
            idle_ttl: 10 * 60,
        }
    }
}

impl SessionAdmissionConfig {
    /// Reject values that cannot produce a live timeout/reaper schedule.
    pub fn validate(&self) -> Result<(), String> {
        if self.wait_timeout == 0 {
            return Err("agents.defaults.session_admission.wait_timeout must be > 0".to_string());
        }
        if self.idle_ttl == 0 {
            return Err("agents.defaults.session_admission.idle_ttl must be > 0".to_string());
        }
        Ok(())
    }
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
            thinking_mode: None,
            session_admission: SessionAdmissionConfig::default(),
        }
    }
}

#[cfg(test)]
mod session_admission_config_tests {
    use super::{AgentDefaults, SessionAdmissionConfig};

    #[test]
    fn missing_session_admission_uses_frozen_defaults() {
        let defaults: AgentDefaults = serde_json::from_value(serde_json::json!({
            "workspace": ".",
            "model": "test",
            "max_tokens": 1024,
            "temperature": 0.1,
            "max_tool_iterations": 3
        }))
        .unwrap();
        assert_eq!(
            defaults.session_admission,
            SessionAdmissionConfig::default()
        );
    }

    #[test]
    fn session_admission_accepts_zero_waiters_but_rejects_zero_durations() {
        let no_waiters = SessionAdmissionConfig {
            max_queue_depth: 0,
            ..SessionAdmissionConfig::default()
        };
        assert!(no_waiters.validate().is_ok());
        assert!(SessionAdmissionConfig {
            wait_timeout: 0,
            ..SessionAdmissionConfig::default()
        }
        .validate()
        .is_err());
        assert!(SessionAdmissionConfig {
            idle_ttl: 0,
            ..SessionAdmissionConfig::default()
        }
        .validate()
        .is_err());
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
    pub feishu: FeishuConfig,
    #[serde(default)]
    pub dingtalk: DingTalkConfig,
    #[serde(default)]
    pub email: EmailConfig,
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
    /// Optional port for webhook mode (not used in WebSocket mode)
    #[serde(default)]
    pub port: Option<u16>,
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
    /// Per-provider reasoning configuration for dynamic model capability detection
    #[serde(default)]
    pub reasoning_config: Option<crate::reasoning::ReasoningConfig>,
    /// Response protocol emitted by an OpenAI-compatible endpoint.
    #[serde(default)]
    pub response_protocol: ProviderResponseProtocol,
}

/// How an OpenAI-compatible endpoint encodes assistant responses.
///
/// `DeepseekV4Dsml` is intentionally opt-in: endpoints that already expose
/// OpenAI JSON tool calls must continue using the default representation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderResponseProtocol {
    #[default]
    OpenaiJson,
    DeepseekV4Dsml,
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
    3000
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Compaction budget configuration — mirrors `BudgetConfig` fields for config file
/// deserialization.  Lives in core so the config layer has no dependency on agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionBudgetConfig {
    /// Maximum tokens allowed in the full assembled context.
    #[serde(default = "default_compaction_max_tokens")]
    pub max_tokens: usize,
    /// Fraction of `max_tokens` reserved for system prompt. Range [0.0, 1.0).
    #[serde(default = "default_compaction_system_budget_ratio")]
    pub system_budget_ratio: f64,
    /// Fraction of history budget that triggers compaction. Range (0.0, 1.0].
    #[serde(default = "default_compaction_threshold_ratio")]
    pub compact_threshold_ratio: f64,
    /// Number of recent messages to always keep (never compacted).
    #[serde(default = "default_compaction_keep_recent_count")]
    pub keep_recent_count: usize,
}

fn default_compaction_max_tokens() -> usize {
    180_000
}
fn default_compaction_system_budget_ratio() -> f64 {
    0.15
}
fn default_compaction_threshold_ratio() -> f64 {
    0.80
}
fn default_compaction_keep_recent_count() -> usize {
    10
}

impl Default for CompactionBudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_compaction_max_tokens(),
            system_budget_ratio: default_compaction_system_budget_ratio(),
            compact_threshold_ratio: default_compaction_threshold_ratio(),
            keep_recent_count: default_compaction_keep_recent_count(),
        }
    }
}

/// Tools configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub builtin: BuiltInToolsConfig,
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
    /// Context compaction budget configuration.
    #[serde(default)]
    pub budget: CompactionBudgetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltInToolsConfig {
    #[serde(default = "default_enabled")]
    pub filesystem: bool,
    #[serde(default = "default_enabled")]
    pub shell: bool,
    #[serde(default = "default_enabled")]
    pub web_search: bool,
    #[serde(default = "default_enabled")]
    pub web_fetch: bool,
    #[serde(default = "default_enabled")]
    pub spawn: bool,
    #[serde(default)]
    pub cron: bool,
    #[serde(default = "default_enabled")]
    pub mcp: bool,
    #[serde(default = "default_enabled")]
    pub attachment: bool,
    #[serde(default)]
    pub planning: bool,
    #[serde(default = "default_enabled")]
    pub enqueue_background_task: bool,
    #[serde(default = "default_enabled")]
    pub update_plan: bool,
    #[serde(default = "default_enabled")]
    pub ask_user: bool,
    #[serde(default = "default_enabled")]
    pub memory: bool,
    #[serde(default = "default_enabled")]
    pub working_memory: bool,
    /// CORE discovery protocol for authorized deferred tools.
    #[serde(default = "default_enabled")]
    pub tool_discovery: bool,
}

impl Default for BuiltInToolsConfig {
    fn default() -> Self {
        Self {
            filesystem: true,
            shell: true,
            web_search: true,
            web_fetch: true,
            spawn: true,
            cron: false,
            mcp: true,
            attachment: true,
            planning: false,
            enqueue_background_task: true,
            update_plan: true,
            ask_user: true,
            memory: true,
            working_memory: true,
            tool_discovery: true,
        }
    }
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

/// Mate (desktop avatar) configuration
///
/// Controls the Diva Mate feature: 3D avatar rendering,
/// voice interaction (TTS/ASR), and model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MateConfig {
    /// Master switch: show/hide Diva Mate sidebar entry
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Selected VRM model filename (relative to public/vrm/models/)
    #[serde(default)]
    pub vrm_model: String,
    /// Whether TTS auto-play is enabled
    #[serde(default)]
    pub tts_enabled: bool,
    /// Whether ASR / microphone input is enabled
    #[serde(default = "default_true")]
    pub asr_enabled: bool,
    /// ASR provider. Currently "web_speech" is the implemented path.
    #[serde(default = "default_asr_provider")]
    pub asr_provider: String,
    /// BCP-47 ASR language tag.
    #[serde(default = "default_asr_language")]
    pub asr_language: String,
    /// API key for remote ASR providers.
    #[serde(default)]
    pub asr_api_key: Option<String>,
    /// Base URL for remote ASR providers.
    #[serde(default)]
    pub asr_base_url: String,
    /// Model for remote ASR providers.
    #[serde(default)]
    pub asr_model: Option<String>,
    /// TTS provider: "browser" | "openai" | "siliconflow" | "minimax"
    #[serde(default = "default_tts_provider")]
    pub tts_provider: String,
    /// Legacy shared API key for remote TTS providers. New GUI code no longer
    /// uses this field and instead stores provider-specific keys below.
    #[serde(default)]
    pub tts_api_key: Option<String>,
    /// API key for OpenAI TTS.
    #[serde(default)]
    pub tts_openai_api_key: Option<String>,
    /// API key for SiliconFlow TTS.
    #[serde(default)]
    pub tts_siliconflow_api_key: Option<String>,
    /// API key for MiniMax TTS.
    #[serde(default)]
    pub tts_minimax_api_key: Option<String>,
    /// Base URL for remote TTS providers.
    #[serde(default)]
    pub tts_base_url: String,
    /// Model for remote TTS providers.
    #[serde(default)]
    pub tts_model: Option<String>,
    /// Provider-specific voice id for system voice selection.
    #[serde(default)]
    pub tts_voice_id: Option<String>,
    /// Relative path under voice_resource/ used as a reference voice.
    #[serde(default)]
    pub tts_reference_voice: Option<String>,
    /// Transcript for the reference voice clip.
    #[serde(default)]
    pub tts_reference_text: Option<String>,
    /// TTS playback speed.
    #[serde(default = "default_tts_speed")]
    pub tts_speed: f64,
    /// TTS playback volume.
    #[serde(default = "default_tts_volume")]
    pub tts_volume: f64,
}

fn default_asr_provider() -> String {
    "web_speech".to_string()
}

fn default_asr_language() -> String {
    "zh-CN".to_string()
}

fn default_tts_provider() -> String {
    "browser".to_string()
}

fn default_tts_speed() -> f64 {
    1.0
}

fn default_tts_volume() -> f64 {
    1.0
}

impl Default for MateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vrm_model: String::new(),
            tts_enabled: false,
            asr_enabled: true,
            asr_provider: default_asr_provider(),
            asr_language: default_asr_language(),
            asr_api_key: None,
            asr_base_url: String::new(),
            asr_model: None,
            tts_provider: default_tts_provider(),
            tts_api_key: None,
            tts_openai_api_key: None,
            tts_siliconflow_api_key: None,
            tts_minimax_api_key: None,
            tts_base_url: String::new(),
            tts_model: None,
            tts_voice_id: None,
            tts_reference_voice: None,
            tts_reference_text: None,
            tts_speed: default_tts_speed(),
            tts_volume: default_tts_volume(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SubAgentStatus tests ───────────────────────────────────────────────

    #[test]
    fn subagent_status_variants_serialize_correctly() {
        let cases = vec![
            (SubAgentStatus::Ok, "\"ok\""),
            (SubAgentStatus::Error, "\"error\""),
            (SubAgentStatus::Timeout, "\"timeout\""),
            (SubAgentStatus::Cancelled, "\"cancelled\""),
        ];
        for (variant, expected_json) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json, "failed for {:?}", variant);

            let back: SubAgentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn subagent_status_unknown_variant_errors() {
        let result = serde_json::from_str::<SubAgentStatus>("\"unknown\"");
        assert!(result.is_err());
    }

    // ── SubAgentResult tests ───────────────────────────────────────────────

    #[test]
    fn subagent_result_round_trips_json() {
        let result = SubAgentResult {
            task_id: "task-001".to_string(),
            status: SubAgentStatus::Ok,
            summary: Some("Completed analysis".to_string()),
            elapsed_ms: 1523,
            tool_call_count: 7,
            token_usage: Some(TokenUsage {
                prompt_tokens: 1200,
                completion_tokens: 340,
                total_tokens: 1540,
            }),
            tool_trace: Some(vec!["read_file".to_string(), "write_file".to_string()]),
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        let back: SubAgentResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back, result);
    }

    #[test]
    fn subagent_result_minimal_json() {
        let result = SubAgentResult {
            task_id: "t1".to_string(),
            status: SubAgentStatus::Error,
            summary: None,
            elapsed_ms: 0,
            tool_call_count: 0,
            token_usage: None,
            tool_trace: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let back: SubAgentResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back, result);
    }

    #[test]
    fn subagent_result_deserializes_from_partial_json() {
        // Simulates a provider that omits optional fields entirely
        let json = r#"{
            "task_id": "t2",
            "status": "timeout",
            "elapsed_ms": 30000,
            "tool_call_count": 3
        }"#;
        let result: SubAgentResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.task_id, "t2");
        assert_eq!(result.status, SubAgentStatus::Timeout);
        assert_eq!(result.summary, None);
        assert_eq!(result.token_usage, None);
        assert_eq!(result.tool_trace, None);
    }

    // ── BatchSpawnRequest tests ────────────────────────────────────────────

    #[test]
    fn batch_spawn_request_multiple_tasks() {
        let req = BatchSpawnRequest {
            tasks: vec![
                SubAgentTask {
                    id: "a".to_string(),
                    goal: "Summarize the file".to_string(),
                    context: None,
                },
                SubAgentTask {
                    id: "b".to_string(),
                    goal: "Find bugs".to_string(),
                    context: Some("Focus on edge cases".to_string()),
                },
                SubAgentTask {
                    id: "c".to_string(),
                    goal: "Write tests".to_string(),
                    context: None,
                },
            ],
            max_iterations: None,
        };

        let json = serde_json::to_string_pretty(&req).unwrap();
        let back: BatchSpawnRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
        assert_eq!(back.tasks.len(), 3);
        assert_eq!(
            back.tasks[1].context.as_deref(),
            Some("Focus on edge cases")
        );
    }

    #[test]
    fn batch_spawn_request_empty_tasks() {
        let req = BatchSpawnRequest {
            tasks: vec![],
            max_iterations: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"tasks":[]}"#);
        let back: BatchSpawnRequest = serde_json::from_str(&json).unwrap();
        assert!(back.tasks.is_empty());
    }

    // ── Legacy tests (kept from original) ──────────────────────────────────

    // ── MaskConfig tests ───────────────────────────────────────────────────

    #[test]
    fn mask_config_parses_without_id() {
        let yaml = r#"
name: "我就是我"
icon: "😊"
"#;
        let config: MaskConfig =
            serde_yaml::from_str(yaml).expect("valid mask config should deserialize");
        assert_eq!(config.id, None);
        assert_eq!(config.name, "我就是我");
    }

    #[test]
    fn mask_config_uses_explicit_id() {
        let config = MaskConfig {
            id: Some("custom-id".to_string()),
            name: "Code Reviewer".to_string(),
            ..Default::default()
        };
        assert_eq!(config.id_or_slug(), "custom-id");
    }

    #[test]
    fn mask_config_slug_for_ascii_name() {
        let config = MaskConfig {
            name: "My Coding Mask".to_string(),
            ..Default::default()
        };
        assert_eq!(config.id_or_slug(), "my-coding-mask");
    }

    #[test]
    fn mask_config_slug_for_chinese_names() {
        let a = MaskConfig {
            name: "代码助手".to_string(),
            ..Default::default()
        };
        let b = MaskConfig {
            name: "研究专家".to_string(),
            ..Default::default()
        };
        let slug_a = a.id_or_slug();
        let slug_b = b.id_or_slug();

        assert!(!slug_a.contains('_'), "slug should not use underscores");
        assert!(!slug_b.contains('_'), "slug should not use underscores");
        assert_ne!(
            slug_a, slug_b,
            "different Chinese names should produce different slugs"
        );
        assert_ne!(slug_a, "____", "slug should not collapse to underscores");
        assert_ne!(slug_b, "____", "slug should not collapse to underscores");
    }
}
