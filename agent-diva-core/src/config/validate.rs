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
    if config.agents.soul.max_chars == 0 {
        errors.push("agents.soul.max_chars must be > 0".to_string());
    }
    if config.agents.soul.frequent_change_window_secs == 0 {
        errors.push("agents.soul.frequent_change_window_secs must be > 0".to_string());
    }
    if config.agents.soul.frequent_change_threshold == 0 {
        errors.push("agents.soul.frequent_change_threshold must be > 0".to_string());
    }

    if config.channels.telegram.enabled && config.channels.telegram.token.trim().is_empty() {
        errors.push("channels.telegram.token is required when telegram is enabled".to_string());
    }
    if config.channels.discord.enabled && config.channels.discord.token.trim().is_empty() {
        errors.push("channels.discord.token is required when discord is enabled".to_string());
    }
    if config.channels.whatsapp.enabled && config.channels.whatsapp.bridge_url.trim().is_empty() {
        errors
            .push("channels.whatsapp.bridge_url is required when whatsapp is enabled".to_string());
    }
    if config.channels.feishu.enabled {
        if config.channels.feishu.app_id.trim().is_empty() {
            errors.push("channels.feishu.app_id is required when feishu is enabled".to_string());
        }
        if config.channels.feishu.app_secret.trim().is_empty() {
            errors
                .push("channels.feishu.app_secret is required when feishu is enabled".to_string());
        }
    }
    if config.channels.dingtalk.enabled {
        if config.channels.dingtalk.client_id.trim().is_empty() {
            errors.push(
                "channels.dingtalk.client_id is required when dingtalk is enabled".to_string(),
            );
        }
        if config.channels.dingtalk.client_secret.trim().is_empty() {
            errors.push(
                "channels.dingtalk.client_secret is required when dingtalk is enabled".to_string(),
            );
        }
    }
    if config.channels.email.enabled {
        if config.channels.email.imap_host.trim().is_empty() {
            errors.push("channels.email.imap_host is required when email is enabled".to_string());
        }
        if config.channels.email.imap_username.trim().is_empty() {
            errors
                .push("channels.email.imap_username is required when email is enabled".to_string());
        }
        if config.channels.email.imap_password.trim().is_empty() {
            errors
                .push("channels.email.imap_password is required when email is enabled".to_string());
        }
        if config.channels.email.smtp_host.trim().is_empty() {
            errors.push("channels.email.smtp_host is required when email is enabled".to_string());
        }
        if config.channels.email.smtp_username.trim().is_empty() {
            errors
                .push("channels.email.smtp_username is required when email is enabled".to_string());
        }
        if config.channels.email.smtp_password.trim().is_empty() {
            errors
                .push("channels.email.smtp_password is required when email is enabled".to_string());
        }
        if config.channels.email.from_address.trim().is_empty() {
            errors
                .push("channels.email.from_address is required when email is enabled".to_string());
        }
    }
    if config.channels.slack.enabled {
        if config.channels.slack.bot_token.trim().is_empty() {
            errors.push("channels.slack.bot_token is required when slack is enabled".to_string());
        }
        if config.channels.slack.app_token.trim().is_empty() {
            errors.push("channels.slack.app_token is required when slack is enabled".to_string());
        }
    }
    if config.channels.qq.enabled {
        if config.channels.qq.app_id.trim().is_empty() {
            errors.push("channels.qq.app_id is required when qq is enabled".to_string());
        }
        if config.channels.qq.secret.trim().is_empty() {
            errors.push("channels.qq.secret is required when qq is enabled".to_string());
        }
    }
    if config.channels.matrix.enabled {
        if config.channels.matrix.homeserver.trim().is_empty() {
            errors
                .push("channels.matrix.homeserver is required when matrix is enabled".to_string());
        }
        if config.channels.matrix.user_id.trim().is_empty() {
            errors.push("channels.matrix.user_id is required when matrix is enabled".to_string());
        }
        if config.channels.matrix.access_token.trim().is_empty() {
            errors.push(
                "channels.matrix.access_token is required when matrix is enabled".to_string(),
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
    if provider != "brave" && provider != "zhipu" {
        errors.push(
            "tools.web.search.provider currently only supports 'brave' or 'zhipu'".to_string(),
        );
    }
    let max_allowed = if provider == "zhipu" { 50 } else { 10 };
    if config.tools.web.search.max_results == 0 || config.tools.web.search.max_results > max_allowed
    {
        errors.push(format!(
            "tools.web.search.max_results must be in [1, {}] when provider='{}'",
            max_allowed,
            if provider.is_empty() {
                "brave"
            } else {
                &provider
            }
        ));
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

        let err = validate_config(&config).unwrap_err();
        assert!(err.to_string().contains("channels.telegram.token"));
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
}
