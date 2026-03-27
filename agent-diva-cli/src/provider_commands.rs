use crate::cli_runtime::{current_provider_name, print_json, set_provider_credentials, CliRuntime};
use agent_diva_core::auth::{ProviderAuthProfile, ProviderAuthService};
use agent_diva_providers::{
    ProviderCatalogService, ProviderLoginMode, ProviderLoginRequest, ProviderLoginService,
};
use anyhow::Result;
use console::style;
use serde::Serialize;

#[derive(Serialize)]
struct ProviderSetReport {
    provider: String,
    model: String,
    config_path: String,
    configured: bool,
    api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderStatusEntry {
    name: String,
    provider: String,
    display_name: String,
    current: bool,
    auth_mode: String,
    login_supported: bool,
    credential_store: String,
    runtime_backend: String,
    configured: bool,
    ready: bool,
    default_model: Option<String>,
    api_base: Option<String>,
    active_profile: Option<String>,
    authenticated: bool,
    expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderStatusReportPayload {
    current_model: String,
    current_provider: Option<String>,
    providers: Vec<ProviderStatusEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderActionReport {
    provider: String,
    profile: Option<String>,
    status: String,
    message: String,
}

pub async fn run_provider_list(runtime: &CliRuntime, json: bool) -> Result<()> {
    let report = build_provider_status_payload(runtime, None).await?;
    if json {
        return print_json(&report.providers);
    }

    println!("{}", style("Providers").bold().cyan());
    println!();
    for provider in report.providers {
        let state = if provider.authenticated {
            style("authenticated").green()
        } else if provider.login_supported {
            style("login required").yellow()
        } else if provider.ready {
            style("configured").green()
        } else {
            style("missing config").yellow()
        };
        let active = if provider.current { " [active]" } else { "" };
        let default_model = provider.default_model.as_deref().unwrap_or("<none>");
        println!(
            "  {} ({}){}",
            provider.provider, provider.display_name, active
        );
        println!("    status: {}", state);
        println!("    auth mode: {}", provider.auth_mode);
        println!("    default model: {}", default_model);
    }

    Ok(())
}

pub async fn run_provider_status(
    runtime: &CliRuntime,
    provider: Option<String>,
    json: bool,
) -> Result<()> {
    let report = build_provider_status_payload(runtime, provider).await?;

    if json {
        return print_json(&report);
    }

    println!("{}", style("Provider Status").bold().cyan());
    println!("  Current model: {}", report.current_model);
    println!(
        "  Current provider: {}",
        report
            .current_provider
            .clone()
            .unwrap_or_else(|| "unresolved".to_string())
    );
    println!();
    for provider in report.providers {
        let state = if provider.authenticated {
            style("authenticated").green()
        } else if provider.login_supported {
            style("not authenticated").yellow()
        } else if provider.ready {
            style("ready").green()
        } else {
            style("missing config").yellow()
        };
        println!("  {}: {}", provider.provider, state);
        println!("    auth mode: {}", provider.auth_mode);
        println!("    login supported: {}", provider.login_supported);
        println!("    credential store: {}", provider.credential_store);
        println!("    runtime backend: {}", provider.runtime_backend);
        if let Some(profile) = &provider.active_profile {
            println!("    active profile: {}", profile);
        }
        if let Some(expires_at) = &provider.expires_at {
            println!("    expires at: {}", expires_at);
        }
        if let Some(default_model) = &provider.default_model {
            println!("    default model: {}", default_model);
        }
    }

    Ok(())
}

pub async fn run_provider_set(
    runtime: &CliRuntime,
    provider: String,
    model: Option<String>,
    api_key: Option<String>,
    api_base: Option<String>,
    json: bool,
) -> Result<()> {
    let provider_name = provider.trim().to_string();
    let catalog = ProviderCatalogService::new();
    let mut config = runtime.load_config()?;
    let view = catalog
        .get_provider_view(&config, &provider_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider '{}'", provider_name))?;
    let selected_model = model
        .or_else(|| view.default_model.clone())
        .or_else(|| {
            (config.agents.defaults.provider.as_deref() == Some(provider_name.as_str()))
                .then(|| config.agents.defaults.model.clone())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Provider '{}' does not expose a default model; pass --model explicitly",
                provider_name
            )
        })?;

    config.agents.defaults.provider = Some(provider_name.clone());
    config.agents.defaults.model = selected_model.clone();
    if config.providers.get(&provider_name).is_some() {
        set_provider_credentials(&mut config, &provider_name, api_key, api_base);
    } else if let Some(custom) = config.providers.get_custom_mut(&provider_name) {
        if let Some(api_key) = api_key {
            custom.api_key = api_key;
        }
        if api_base.is_some() {
            custom.api_base = api_base;
        }
    }
    runtime.loader().save(&config)?;

    let report = ProviderSetReport {
        provider: provider_name.clone(),
        model: selected_model,
        config_path: runtime.config_path().display().to_string(),
        configured: catalog
            .get_provider_view(&config, &provider_name)
            .map(|provider| provider.configured)
            .unwrap_or(false),
        api_base: catalog
            .get_provider_view(&config, &provider_name)
            .and_then(|provider| provider.api_base),
    };

    if json {
        return print_json(&report);
    }

    println!("{}", style("Provider updated.").green().bold());
    println!("  Provider: {}", report.provider);
    println!("  Model: {}", report.model);
    println!("  Config: {}", report.config_path);
    Ok(())
}

pub async fn run_provider_login(
    runtime: &CliRuntime,
    provider: String,
    profile: String,
    device_code: bool,
    paste_code: Option<String>,
    json: bool,
) -> Result<()> {
    let catalog = ProviderCatalogService::new();
    let config = runtime.load_config()?;
    let view = catalog
        .get_provider_view(&config, &provider)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider '{}'", provider))?;
    if !view.login_supported {
        anyhow::bail!(
            "Provider '{}' does not support login (auth_mode={})",
            provider,
            view.auth_mode
        );
    }

    let mode = if device_code {
        ProviderLoginMode::DeviceCode
    } else if paste_code.is_some() {
        ProviderLoginMode::PasteRedirect { input: paste_code }
    } else {
        ProviderLoginMode::Browser
    };
    let service = ProviderLoginService::new();
    let auth = ProviderAuthService::new(runtime.config_dir());
    let report = service
        .login(
            &auth,
            ProviderLoginRequest {
                provider: provider.clone(),
                profile_name: profile.clone(),
                mode,
            },
        )
        .await?;

    if json {
        return print_json(&report);
    }

    println!("{}", style("Provider login completed.").green().bold());
    println!("  Provider: {}", report.provider);
    println!("  Profile: {}", report.profile_name);
    if let Some(account_id) = report.account_id {
        println!("  Account: {}", account_id);
    }
    Ok(())
}

pub async fn run_provider_logout(
    runtime: &CliRuntime,
    provider: String,
    profile: Option<String>,
    json: bool,
) -> Result<()> {
    let auth = ProviderAuthService::new(runtime.config_dir());
    let selected_profile = select_profile_name(&auth, &provider, profile.as_deref()).await?;
    let removed = auth.remove_profile(&provider, &selected_profile).await?;
    let report = ProviderActionReport {
        provider,
        profile: Some(selected_profile),
        status: if removed {
            "logged_out".to_string()
        } else {
            "not_found".to_string()
        },
        message: if removed {
            "Profile removed from auth store".to_string()
        } else {
            "Profile was not present in auth store".to_string()
        },
    };

    if json {
        return print_json(&report);
    }

    println!("{}", style("Provider logout completed.").green().bold());
    println!("  {}", report.message);
    Ok(())
}

pub async fn run_provider_use(
    runtime: &CliRuntime,
    provider: String,
    profile: String,
    json: bool,
) -> Result<()> {
    let auth = ProviderAuthService::new(runtime.config_dir());
    let selected = auth.set_active_profile(&provider, &profile).await?;
    let report = ProviderActionReport {
        provider,
        profile: Some(selected.clone()),
        status: "active_profile_updated".to_string(),
        message: format!("Active profile set to {}", selected),
    };

    if json {
        return print_json(&report);
    }

    println!("{}", style("Provider profile selected.").green().bold());
    println!("  {}", report.message);
    Ok(())
}

pub async fn run_provider_refresh(
    runtime: &CliRuntime,
    provider: String,
    profile: Option<String>,
    json: bool,
) -> Result<()> {
    if provider != "openai-codex" {
        anyhow::bail!(
            "Provider '{}' does not support refresh in this build",
            provider
        );
    }
    let auth = ProviderAuthService::new(runtime.config_dir());
    let refreshed = auth.refresh_openai_codex_tokens(profile.as_deref()).await?;
    let report = ProviderActionReport {
        provider,
        profile: Some(refreshed.profile_name.clone()),
        status: "refreshed".to_string(),
        message: "OAuth token refreshed".to_string(),
    };

    if json {
        return print_json(&report);
    }

    println!("{}", style("Provider token refreshed.").green().bold());
    println!("  Profile: {}", refreshed.profile_name);
    Ok(())
}

pub async fn run_provider_models(
    runtime: &CliRuntime,
    provider: String,
    json: bool,
    _static_fallback: bool,
) -> Result<()> {
    let config = runtime.load_config()?;
    let catalog = ProviderCatalogService::new()
        .list_provider_models(&config, &provider, true, None)
        .await
        .map_err(anyhow::Error::msg)?;

    if json {
        let model_entries = catalog.models.clone();
        let payload = serde_json::json!({
            "provider": catalog.provider,
            "source": catalog.catalog_source,
            "catalog_source": catalog.catalog_source,
            "runtime_supported": catalog.runtime_supported,
            "api_base": catalog.api_base,
            "models": model_entries.iter().map(|entry| entry.id.clone()).collect::<Vec<_>>(),
            "model_entries": model_entries,
            "custom_models": catalog.custom_models,
            "warnings": catalog.warnings,
            "error": catalog.error,
        });
        return print_json(&payload);
    }

    println!("{}", style("Provider Models").bold().cyan());
    println!("  Provider: {}", catalog.provider);
    println!("  Source: {}", catalog.catalog_source);
    if let Some(api_base) = &catalog.api_base {
        println!("  API base: {}", api_base);
    }
    if let Some(error) = &catalog.error {
        println!("  Error: {}", style(error).red());
    }
    for warning in &catalog.warnings {
        println!("  Warning: {}", style(warning).yellow());
    }
    if catalog.models.is_empty() {
        println!("  Models: <none>");
    } else {
        println!("  Models:");
        for model in catalog.models {
            let source = match model.source {
                agent_diva_providers::ProviderModelSource::Runtime => "runtime",
                agent_diva_providers::ProviderModelSource::Static => "static",
                agent_diva_providers::ProviderModelSource::Custom => "custom",
            };
            println!("    - {} ({})", model.id, source);
        }
    }

    Ok(())
}

async fn build_provider_status_payload(
    runtime: &CliRuntime,
    provider_filter: Option<String>,
) -> Result<ProviderStatusReportPayload> {
    let config = runtime.load_config()?;
    let current_provider = current_provider_name(&config);
    let auth = ProviderAuthService::new(runtime.config_dir());
    let catalog = ProviderCatalogService::new();

    let mut providers = Vec::new();
    for view in catalog.list_provider_views(&config) {
        if provider_filter
            .as_ref()
            .is_some_and(|expected| expected != &view.id)
        {
            continue;
        }
        let active_profile = auth.get_active_profile(&view.id).await?;
        providers.push(build_provider_status_entry(
            &view,
            current_provider.as_deref() == Some(view.id.as_str()),
            active_profile,
        ));
    }

    if providers.is_empty() {
        if let Some(provider) = provider_filter {
            anyhow::bail!("Unknown provider '{}'", provider);
        }
    }

    Ok(ProviderStatusReportPayload {
        current_model: config.agents.defaults.model,
        current_provider,
        providers,
    })
}

fn build_provider_status_entry(
    view: &agent_diva_providers::ProviderView,
    current: bool,
    active_profile: Option<ProviderAuthProfile>,
) -> ProviderStatusEntry {
    let expires_at = active_profile
        .as_ref()
        .and_then(|profile| profile.token_set.as_ref())
        .and_then(|tokens| tokens.expires_at)
        .map(|value| value.to_rfc3339());
    ProviderStatusEntry {
        name: view.id.clone(),
        provider: view.id.clone(),
        display_name: view.display_name.clone(),
        current,
        auth_mode: view.auth_mode.clone(),
        login_supported: view.login_supported,
        credential_store: view.credential_store.clone(),
        runtime_backend: view.runtime_backend.clone(),
        configured: view.configured,
        ready: view.ready,
        default_model: view.default_model.clone(),
        api_base: view.api_base.clone(),
        active_profile: active_profile
            .as_ref()
            .map(|profile| profile.profile_name.clone()),
        authenticated: active_profile.is_some(),
        expires_at,
    }
}

async fn select_profile_name(
    auth: &ProviderAuthService,
    provider: &str,
    requested: Option<&str>,
) -> Result<String> {
    if let Some(requested) = requested {
        return Ok(requested.to_string());
    }
    let active = auth
        .get_active_profile(provider)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No active profile found for provider '{}'", provider))?;
    Ok(active.profile_name)
}
