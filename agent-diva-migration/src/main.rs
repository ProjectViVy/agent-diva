//! Explicit offline migration CLI.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

mod experience;
mod typed_memory;
mod workspace_identity;

#[derive(Parser)]
#[command(name = "agent-diva-migrate", version)]
#[command(about = "Explicit offline migration utility for Agent Diva")]
struct Cli {
    /// Comma-separated migration feature flags. Only `Apply` operations whose
    /// feature is enabled are permitted; `DryRun`/`Rollback` are always allowed.
    #[arg(long, global = true, value_delimiter = ',')]
    features: Vec<String>,
    #[command(subcommand)]
    command: Command,
}

/// Canonical migration feature names, matching the mounted module names.
const FEATURE_EXPERIENCE: &str = "experience";
const FEATURE_MEMORY: &str = "memory";
const FEATURE_WORKSPACE_IDENTITY: &str = "workspace_identity";

/// Reject an `Apply` when its feature flag is not enabled.
fn require_feature(enabled: &[String], name: &str) -> Result<()> {
    if enabled.iter().any(|feature| feature == name) {
        Ok(())
    } else {
        anyhow::bail!(
            "Apply for `{}` is disabled until the `--features {name}` flag is passed",
            name
        )
    }
}

#[derive(Subcommand)]
enum Command {
    /// Import legacy Memory authority into Embedded Laputa.
    Memory {
        #[command(subcommand)]
        operation: MemoryOperation,
    },
    /// Backfill payload-free AutoDream evidence from existing sessions.
    Experience {
        #[command(subcommand)]
        operation: ExperienceOperation,
    },
}

#[derive(Subcommand)]
enum ExperienceOperation {
    /// Inspect eligible tool-result records without writing.
    DryRun(ExperienceArgs),
    /// Persist a manifest and append eligible evidence.
    Apply(ExperienceArgs),
    /// Remove only evidence listed by the migration manifest.
    Rollback(ExperienceRollbackArgs),
}

#[derive(Debug, Args)]
struct ExperienceArgs {
    /// Target Agent Diva workspace containing sessions/.
    #[arg(long)]
    workspace: PathBuf,
}

#[derive(Debug, Args)]
struct ExperienceRollbackArgs {
    #[arg(long)]
    workspace: PathBuf,
    #[arg(long)]
    migration_id: String,
}

#[derive(Subcommand)]
enum MemoryOperation {
    /// Validate and report without writing migration artifacts or records.
    DryRun(ImportArgs),
    /// Create a verified backup and atomically import records.
    Apply(ImportArgs),
    /// Restore the pre-import database recorded by a migration manifest.
    Rollback(RollbackArgs),
    /// Normalize the store's workspace identity without changing Memory content.
    Identity {
        #[command(subcommand)]
        operation: IdentityOperation,
    },
}

#[derive(Subcommand)]
enum IdentityOperation {
    /// Report whether an identity-only migration is required.
    DryRun(IdentityArgs),
    /// Create a verified backup and apply the canonical identity.
    Apply(IdentityArgs),
    /// Restore the verified pre-migration identity from its manifest.
    Rollback(IdentityArgs),
}

#[derive(Debug, Args)]
struct IdentityArgs {
    /// Target Agent Diva workspace.
    #[arg(long)]
    workspace: PathBuf,
}

#[derive(Debug, Args)]
struct ImportArgs {
    /// Root that must contain every explicitly selected source.
    #[arg(long)]
    source_root: PathBuf,
    /// Explicit MEMORY.md, HISTORY.md, or supported Laputa section JSON.
    #[arg(long = "source", required = true)]
    sources: Vec<PathBuf>,
    /// Target Agent Diva workspace.
    #[arg(long)]
    workspace: PathBuf,
    /// Tenant identifier assigned to imported records.
    #[arg(long, default_value = "local")]
    tenant_id: String,
    /// Stable workspace identifier stored in Embedded Laputa.
    #[arg(long)]
    workspace_id: Option<String>,
}

#[derive(Debug, Args)]
struct RollbackArgs {
    #[arg(long)]
    workspace: PathBuf,
    #[arg(long)]
    workspace_id: Option<String>,
    #[arg(long)]
    migration_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let features = &cli.features;
    let report = match cli.command {
        Command::Memory { operation } => match operation {
            MemoryOperation::DryRun(args) => {
                serde_json::to_value(typed_memory::dry_run(&request(args)).await?)?
            }
            MemoryOperation::Apply(args) => {
                require_feature(features, FEATURE_MEMORY)?;
                serde_json::to_value(typed_memory::apply(&request(args)).await?)?
            }
            MemoryOperation::Rollback(args) => {
                let workspace_id = args.workspace_id.unwrap_or_else(|| {
                    agent_diva_core::workspace_identity::canonical_workspace_id(&args.workspace)
                });
                serde_json::to_value(
                    typed_memory::rollback(&args.workspace, &workspace_id, &args.migration_id)
                        .await?,
                )?
            }
            MemoryOperation::Identity { operation } => match operation {
                IdentityOperation::DryRun(args) => {
                    serde_json::to_value(workspace_identity::dry_run(&args.workspace).await?)?
                }
                IdentityOperation::Apply(args) => {
                    require_feature(features, FEATURE_WORKSPACE_IDENTITY)?;
                    serde_json::to_value(workspace_identity::apply(&args.workspace).await?)?
                }
                IdentityOperation::Rollback(args) => {
                    serde_json::to_value(workspace_identity::rollback(&args.workspace).await?)?
                }
            },
        },
        Command::Experience { operation } => serde_json::to_value(match operation {
            ExperienceOperation::DryRun(args) => experience::dry_run(&args.workspace)?,
            ExperienceOperation::Apply(args) => {
                require_feature(features, FEATURE_EXPERIENCE)?;
                experience::apply(&args.workspace)?
            }
            ExperienceOperation::Rollback(args) => {
                experience::rollback(&args.workspace, &args.migration_id)?
            }
        })?,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn request(args: ImportArgs) -> typed_memory::MemoryImportRequest {
    let workspace_id = args.workspace_id.unwrap_or_else(|| {
        agent_diva_core::workspace_identity::canonical_workspace_id(&args.workspace)
    });
    typed_memory::MemoryImportRequest {
        source_root: args.source_root,
        sources: args.sources,
        workspace: args.workspace,
        tenant_id: args.tenant_id,
        workspace_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_feature_rejects_when_flag_disabled() {
        let err = require_feature(&[], FEATURE_MEMORY).unwrap_err();
        assert!(err.to_string().contains("is disabled"));
        assert!(err.to_string().contains(FEATURE_MEMORY));
    }

    #[test]
    fn require_feature_allows_when_flag_enabled() {
        let enabled = vec![FEATURE_MEMORY.to_string(), FEATURE_EXPERIENCE.to_string()];
        assert!(require_feature(&enabled, FEATURE_MEMORY).is_ok());
        assert!(require_feature(&enabled, FEATURE_EXPERIENCE).is_ok());
        assert!(
            require_feature(&enabled, FEATURE_WORKSPACE_IDENTITY).is_err(),
            "unlisted feature must stay disabled"
        );
    }

    #[test]
    fn require_feature_is_exact_match() {
        let enabled = vec!["memory".to_string()];
        assert!(require_feature(&enabled, "memory").is_ok());
        assert!(
            require_feature(&enabled, "MEMORY").is_err(),
            "feature match must be case-sensitive"
        );
    }
}
