//! FullSwarm 序曲配置：工作区根目录 `swarm-prelude.toml` 或 `swarm-prelude.yaml` / `.yml`。
//!
//! **冻结路径（维护者单点）：** `<workspace>/swarm-prelude.toml`（优先），其次
//! `<workspace>/swarm-prelude.yaml`、`<workspace>/swarm-prelude.yml`。未提供文件时使用
//! 与阶段 A 硬编码行为一致的默认值。

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::warn;

/// 配置文件名（TOML，优先加载）。
pub const SWARM_PRELUDE_FILE_TOML: &str = "swarm-prelude.toml";
/// 备选：YAML。
pub const SWARM_PRELUDE_FILE_YAML: &str = "swarm-prelude.yaml";
/// 备选：`.yml` 扩展名。
pub const SWARM_PRELUDE_FILE_YML: &str = "swarm-prelude.yml";

const DEFAULT_PLANNER_SYSTEM: &str =
    "你是多智能体蜂群中的「规划代理」。只输出条理清晰的要点与建议步骤，不要寒暄，不要自称 AI。";
const DEFAULT_CRITIC_SYSTEM: &str =
    "你是蜂群中的「批评/风险代理」。针对上一条「规划代理」的输出，指出盲区、风险与需补充的验证点。用简洁列表。";

const DEFAULT_SUMMARY_PREAMBLE: &str = "（以下为同一 turn 内蜂群成员间交流摘要，供你综合后回答用户；勿向用户逐字复述代理角色标签。）";

fn default_schema_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

fn default_max_prelude_rounds() -> u32 {
    2
}

fn default_summary_preamble() -> String {
    DEFAULT_SUMMARY_PREAMBLE.to_string()
}

fn default_roles() -> Vec<SwarmPreludeRole> {
    vec![
        SwarmPreludeRole {
            phase_id: "swarm_peer_planner".to_string(),
            phase_label: "蜂群 · 规划代理正在整理思路".to_string(),
            system_prompt: DEFAULT_PLANNER_SYSTEM.to_string(),
            input: PreludeInputSource::OriginalUser,
            max_tokens: 768,
            temperature: 0.4,
            summary_section_title: Some("【规划摘要】".to_string()),
        },
        SwarmPreludeRole {
            phase_id: "swarm_peer_critic".to_string(),
            phase_label: "蜂群 · 批评代理正在回应规划".to_string(),
            system_prompt: DEFAULT_CRITIC_SYSTEM.to_string(),
            input: PreludeInputSource::PreviousOutput,
            max_tokens: 768,
            temperature: 0.5,
            summary_section_title: Some("【批评与补充】".to_string()),
        },
    ]
}

fn default_merge_phase() -> SwarmPreludeMergePhase {
    SwarmPreludeMergePhase {
        enabled: true,
        phase_id: "swarm_peer_merge".to_string(),
        phase_label: "蜂群 · 内部交流已收敛，主代理将综合答复".to_string(),
    }
}

/// 单角色一步：一次 `LLMProvider::chat`（无工具）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmPreludeRole {
    pub phase_id: String,
    pub phase_label: String,
    pub system_prompt: String,
    #[serde(default)]
    pub input: PreludeInputSource,
    #[serde(default = "default_role_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_role_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub summary_section_title: Option<String>,
}

fn default_role_max_tokens() -> u32 {
    768
}

fn default_role_temperature() -> f64 {
    0.4
}

/// 用户消息来源：首轮用户原文，或上一角色模型输出。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreludeInputSource {
    #[default]
    OriginalUser,
    PreviousOutput,
}

/// 序曲结束后可选的「合并」阶段过程事件（与阶段 A 一致）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmPreludeMergePhase {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub phase_id: String,
    pub phase_label: String,
}

impl Default for SwarmPreludeMergePhase {
    fn default() -> Self {
        default_merge_phase()
    }
}

/// 工作区序曲配置（`schema_version` 供 NFR-I2 演进）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmPreludeConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 序曲内最多执行的 **角色步数**（LLM 调用次数）；达到上限后不再调用后续角色，并发出过程事件。
    #[serde(default = "default_max_prelude_rounds")]
    pub max_prelude_rounds: u32,
    #[serde(default)]
    pub roles: Vec<SwarmPreludeRole>,
    #[serde(default = "default_summary_preamble")]
    pub summary_preamble: String,
    #[serde(default = "default_merge_phase")]
    pub merge_phase: SwarmPreludeMergePhase,
}

impl Default for SwarmPreludeConfig {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            enabled: true,
            max_prelude_rounds: default_max_prelude_rounds(),
            roles: default_roles(),
            summary_preamble: default_summary_preamble(),
            merge_phase: default_merge_phase(),
        }
    }
}

impl SwarmPreludeConfig {
    /// 与阶段 A 完全一致的内建默认（含两角色文案与参数）。
    #[must_use]
    pub fn phase_a_equivalent() -> Self {
        Self::default()
    }

    fn normalize(mut self) -> Self {
        if self.schema_version != 1 {
            warn!(
                schema_version = self.schema_version,
                "swarm prelude schema_version != 1; still applying loaded config"
            );
        }
        if self.enabled && self.roles.is_empty() {
            warn!("swarm prelude enabled but roles empty; using built-in default roles");
            self.roles = default_roles();
        }
        self
    }
}

fn parse_prelude_toml(raw: &str) -> Result<SwarmPreludeConfig, String> {
    toml::from_str::<SwarmPreludeConfig>(raw).map_err(|e| e.to_string())
}

fn parse_prelude_yaml(raw: &str) -> Result<SwarmPreludeConfig, String> {
    serde_yaml::from_str::<SwarmPreludeConfig>(raw).map_err(|e| e.to_string())
}

/// 从工作区根加载序曲配置：优先 `swarm-prelude.toml`，其次 `.yaml` / `.yml`。
/// 文件不存在则返回 [`SwarmPreludeConfig::default`]。解析或读取失败时记录 `warn!` 并回退默认。
#[must_use]
pub fn load_swarm_prelude_config_from_workspace(workspace: &Path) -> SwarmPreludeConfig {
    let candidates = [
        workspace.join(SWARM_PRELUDE_FILE_TOML),
        workspace.join(SWARM_PRELUDE_FILE_YAML),
        workspace.join(SWARM_PRELUDE_FILE_YML),
    ];
    for path in &candidates {
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match std::fs::read_to_string(path) {
            Ok(raw) => {
                let parsed = if ext == "toml" {
                    parse_prelude_toml(&raw)
                } else {
                    parse_prelude_yaml(&raw)
                };
                match parsed {
                    Ok(c) => return c.normalize(),
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            err = %e,
                            "swarm prelude config parse failed; using defaults"
                        );
                        return SwarmPreludeConfig::default();
                    }
                }
            }
            Err(e) => {
                warn!(
                    path = %path.display(),
                    err = %e,
                    "swarm prelude config unreadable; using defaults"
                );
                return SwarmPreludeConfig::default();
            }
        }
    }
    SwarmPreludeConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_phase_a_two_roles() {
        let c = SwarmPreludeConfig::default();
        assert!(c.enabled);
        assert_eq!(c.max_prelude_rounds, 2);
        assert_eq!(c.roles.len(), 2);
        assert_eq!(c.roles[0].input, PreludeInputSource::OriginalUser);
        assert_eq!(c.roles[1].input, PreludeInputSource::PreviousOutput);
        assert_eq!(c.roles[0].system_prompt, DEFAULT_PLANNER_SYSTEM);
        assert_eq!(c.roles[1].system_prompt, DEFAULT_CRITIC_SYSTEM);
        assert_eq!(c.roles[0].max_tokens, 768);
        assert_eq!(c.roles[1].temperature, 0.5);
    }

    #[test]
    fn parse_toml_enabled_false() {
        let raw = r#"
schema_version = 1
enabled = false
"#;
        let c = parse_prelude_toml(raw).unwrap();
        assert!(!c.enabled);
    }

    #[test]
    fn parse_yaml_max_rounds_and_cap_schema() {
        let raw = r#"
schema_version: 1
enabled: true
max_prelude_rounds: 1
roles:
  - phase_id: a
    phase_label: L
    system_prompt: S
    input: original_user
    max_tokens: 100
    temperature: 0.1
    summary_section_title: T
"#;
        let c = parse_prelude_yaml(raw).unwrap().normalize();
        assert_eq!(c.max_prelude_rounds, 1);
        assert_eq!(c.roles.len(), 1);
    }

    #[test]
    fn load_missing_file_uses_default() {
        let dir = tempfile::tempdir().unwrap();
        let c = load_swarm_prelude_config_from_workspace(dir.path());
        assert_eq!(c, SwarmPreludeConfig::default());
    }

    #[test]
    fn load_valid_toml_from_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(SWARM_PRELUDE_FILE_TOML),
            "schema_version = 1\nenabled = false\n",
        )
        .unwrap();
        let c = load_swarm_prelude_config_from_workspace(dir.path());
        assert!(!c.enabled);
    }
}
