use crate::cli_runtime::{
    print_json, provider_status_report, provider_statuses, set_provider_credentials, CliRuntime,
};
use agent_diva_providers::ProviderCatalogService;
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

#[derive(Serialize)]
struct ProviderLoginReport {
    provider: String,
    status: String,
    message: String,
}

pub async fn run_provider_list(runtime: &CliRuntime, json: bool) -> Result<()> {
    let config = runtime.load_config()?;
    let providers = provider_statuses(&config);

    if json {
        return print_json(&providers);
    }

    println!("{}", style("Providers").bold().cyan());
    println!();
    for provider in providers {
        let status = if provider.ready {
            style("configured").green()
        } else {
            style("missing config").yellow()
        };
        let active = if provider.current { " [active]" } else { "" };
        let default_model = provider
            .default_model
            .as_deref()
            .unwrap_or("<explicit --model required>");
        println!("  {} ({}){}", provider.name, provider.display_name, active);
        println!("    status: {}", status);
        println!("    default model: {}", default_model);
    }

    Ok(())
}

pub async fn run_provider_status(runtime: &CliRuntime, json: bool) -> Result<()> {
    let config = runtime.load_config()?;
    let report = provider_status_report(&config);

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
        let status = if provider.ready {
            style("ready").green()
        } else {
            style("missing fields").yellow()
        };
        println!("  {}: {}", provider.name, status);
        if !provider.missing_fields.is_empty() {
            println!("    missing: {}", provider.missing_fields.join(", "));
        }
        if let Some(default_model) = provider.default_model {
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

pub async fn run_provider_login(provider: String, json: bool) -> Result<()> {
    let report = ProviderLoginReport {
        provider: provider.clone(),
        status: "not_implemented".to_string(),
        message: format!(
            "provider login for '{}' is a placeholder in this build; implement OAuth/device flow per provider later",
            provider
        ),
    };

    if json {
        return print_json(&report);
    }

    println!(
        "{}",
        style("Provider login not implemented.").yellow().bold()
    );
    println!("  {}", report.message);
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
