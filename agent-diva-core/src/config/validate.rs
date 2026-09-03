//! Configuration validation rules.

use super::schema::Config;

/// Validate configuration and return aggregated validation errors.
pub fn validate_config(config: &Config) -> crate::Result<()> {
    let mut errors = Vec::new();

    if config.agents.defaults.workspace.trim().is_empty() {
        errors.push("agents.defaults.workspace must not be empty".to_string());
    }
    if config.agents.defaults.max_tokens == 0 {
        errors.push("agents.defaults.max_tokens must be > 0".to_string());
    }
    if !(0.0..=2.0).contains(&config.agents.defaults.temperature) {
        errors.push("agents.defaults.temperature must be in [0.0, 2.0]".to_string());
    }
    if config.agents.defaults.max_tool_iterations == 0 {
        errors.push("agents.defaults.max_tool_iterations must be > 0".to_string());
    }
    if let Some(reasoning_effort) = &config.agents.defaults.reasoning_effort {
        let effort = reasoning_effort.trim().to_lowercase();
        if !effort.is_empty() && effort != "low" && effort != "medium" && effort != "high" {
            errors.push(
                "agents.defaults.reasoning_effort must be one of: low, medium, high".to_string(),
            );
        }
    }

    for (name, server) in &config.tools.mcp_servers {
        let has_stdio = !server.command.trim().is_empty();
        let has_http = !server.url.trim().is_empty();
        if !has_stdio && !has_http {
            errors.push(format!(
                "tools.mcp_servers.{} must set either command (stdio) or url (http)",
                name
            ));
        }
    }

    let provider = config.tools.web.search.provider.trim().to_lowercase();
    if provider != "brave" && provider != "bocha" && provider != "zhipu" {
        errors.push(
            "tools.web.search.provider currently only supports 'brave', 'bocha', or 'zhipu'"
                .to_string(),
        );
    }
    let max_allowed = if provider == "zhipu" || provider == "bocha" {
        50
    } else {
        10
    };
    if config.tools.web.search.max_results == 0 || config.tools.web.search.max_results > max_allowed
    {
        errors.push(format!(
            "tools.web.search.max_results must be in [1, {}] when provider='{}'",
            max_allowed,
            if provider.is_empty() {
                "bocha"
            } else {
                &provider
            }
        ));
    }

    let asr_provider = config.mate.asr_provider.trim().to_lowercase();
    if !asr_provider.is_empty() && asr_provider != "web_speech" && asr_provider != "siliconflow" {
        errors.push("mate.asr_provider must be one of: web_speech, siliconflow".to_string());
    }
    let tts_provider = config.mate.tts_provider.trim().to_lowercase();
    if !tts_provider.is_empty()
        && tts_provider != "browser"
        && tts_provider != "openai"
        && tts_provider != "siliconflow"
        && tts_provider != "minimax"
    {
        errors.push(
            "mate.tts_provider must be one of: browser, openai, siliconflow, minimax".to_string(),
        );
    }
    if !config.mate.tts_speed.is_finite() || config.mate.tts_speed <= 0.0 {
        errors.push("mate.tts_speed must be > 0".to_string());
    }
    if !config.mate.tts_volume.is_finite()
        || config.mate.tts_volume < 0.0
        || config.mate.tts_volume > 2.0
    {
        errors.push("mate.tts_volume must be in [0.0, 2.0]".to_string());
    }

    if let Err(error) = config.reports.llm_curation.validate() {
        errors.push(error);
    }

    if config.channels.email.enabled && !config.channels.email.consent_granted {
        errors.push("channels.email.consent_granted must be true when enabled".to_string());
    }
    let mut require = |enabled: bool, channel: &str, field: &str, value: &str| {
        if enabled && value.trim().is_empty() {
            errors.push(format!(
                "channels.{channel}.{field} must not be empty when enabled"
            ));
        }
    };
    require(
        config.channels.telegram.enabled,
        "telegram",
        "token",
        &config.channels.telegram.token,
    );
    require(
        config.channels.discord.enabled,
        "discord",
        "token",
        &config.channels.discord.token,
    );
    for (field, value) in [
        ("app_id", config.channels.feishu.app_id.as_str()),
        ("app_secret", config.channels.feishu.app_secret.as_str()),
    ] {
        require(config.channels.feishu.enabled, "feishu", field, value);
    }
    for (field, value) in [
        ("client_id", config.channels.dingtalk.client_id.as_str()),
        (
            "client_secret",
            config.channels.dingtalk.client_secret.as_str(),
        ),
    ] {
        require(config.channels.dingtalk.enabled, "dingtalk", field, value);
    }
    for (field, value) in [
        ("imap_host", config.channels.email.imap_host.as_str()),
        (
            "imap_username",
            config.channels.email.imap_username.as_str(),
        ),
        (
            "imap_password",
            config.channels.email.imap_password.as_str(),
        ),
        ("smtp_host", config.channels.email.smtp_host.as_str()),
        (
            "smtp_username",
            config.channels.email.smtp_username.as_str(),
        ),
        (
            "smtp_password",
            config.channels.email.smtp_password.as_str(),
        ),
        ("from_address", config.channels.email.from_address.as_str()),
    ] {
        require(config.channels.email.enabled, "email", field, value);
    }
    for (field, value) in [
        ("app_id", config.channels.qq.app_id.as_str()),
        ("secret", config.channels.qq.secret.as_str()),
    ] {
        require(config.channels.qq.enabled, "qq", field, value);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(crate::Error::Validation(errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_accepts_defaults() {
        let mut config = Config::default();
        config.providers.anthropic.api_key = "test-key".to_string();
        validate_config(&config).unwrap();
    }

    #[test]
    fn test_validate_enabled_channel_requires_credentials() {
        let mut config = Config::default();
        config.channels.telegram.enabled = true;
        config.providers.anthropic.api_key = "test-key".to_string();

        let error = validate_config(&config).unwrap_err();
        assert!(error.to_string().contains("channels.telegram.token"));
    }

    #[test]
    fn test_validate_mcp_server_requires_transport() {
        let mut config = Config::default();
        config.tools.mcp_servers.insert(
            "bad".to_string(),
            super::super::schema::MCPServerConfig::default(),
        );

        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("tools.mcp_servers.bad"));
    }

    #[test]
    fn test_validate_bocha_accepts_higher_max_results() {
        let mut config = Config::default();
        config.providers.anthropic.api_key = "test-key".to_string();
        config.tools.web.search.provider = "bocha".to_string();
        config.tools.web.search.max_results = 50;

        validate_config(&config).unwrap();
    }

    #[test]
    fn test_validate_accepts_minimax_tts_provider() {
        let mut config = Config::default();
        config.providers.anthropic.api_key = "test-key".to_string();
        config.mate.tts_provider = "minimax".to_string();

        validate_config(&config).unwrap();
    }

    #[test]
    fn test_validate_accepts_siliconflow_asr_provider() {
        let mut config = Config::default();
        config.providers.anthropic.api_key = "test-key".to_string();
        config.mate.asr_provider = "siliconflow".to_string();

        validate_config(&config).unwrap();
    }
}
