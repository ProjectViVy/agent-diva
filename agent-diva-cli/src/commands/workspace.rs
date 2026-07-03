use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

use agent_diva_core::config::Config;
use crate::cli_runtime::CliRuntime;

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceCommands {
    /// List all workspaces
    List,
    /// Create a new workspace
    Create {
        /// Workspace name
        name: String,
        /// Optional source path to copy/symlink from
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Switch to a different workspace
    Switch {
        /// Workspace name
        name: String,
    },
    /// Delete a workspace
    Delete {
        /// Workspace name
        name: String,
        /// Force deletion without confirmation
        #[arg(long)]
        force: bool,
    },
}

pub async fn run(command: WorkspaceCommands, runtime: &CliRuntime) -> Result<()> {
    let workspaces_root = runtime.config_dir().join("workspaces");
    std::fs::create_dir_all(&workspaces_root)?;

    match command {
        WorkspaceCommands::List => {
            let entries = std::fs::read_dir(&workspaces_root)
                .with_context(|| format!("Failed to read workspaces dir: {}", workspaces_root.display()))?;

            let mut names: Vec<String> = Vec::new();
            for entry in entries {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        names.push(name.to_string());
                    }
                }
            }

            if names.is_empty() {
                println!("No workspaces found.");
            } else {
                names.sort();
                println!("Workspaces:");
                for name in names {
                    println!("  {}", name);
                }
            }
        }
        WorkspaceCommands::Create { name, path } => {
            let target = workspaces_root.join(&name);
            if target.exists() {
                anyhow::bail!("Workspace '{}' already exists.", name);
            }

            if let Some(src) = path {
                // Copy directory contents from source
                copy_dir_all(&src, &target)?;
                println!("Created workspace '{}' from {}", name, src.display());
            } else {
                std::fs::create_dir_all(&target)?;
                println!("Created workspace '{}' at {}", name, target.display());
            }
        }
        WorkspaceCommands::Switch { name } => {
            let target = workspaces_root.join(&name);
            if !target.exists() {
                anyhow::bail!("Workspace '{}' does not exist.", name);
            }

            let mut config: Config = runtime.load_config()?;
            let workspace_path = target.display().to_string();
            config.agents.defaults.workspace = workspace_path;
            runtime.loader().save(&config)?;

            println!("Switched to workspace '{}'.", name);
            println!("Restart gateway for changes to take effect.");
        }
        WorkspaceCommands::Delete { name, force } => {
            let target = workspaces_root.join(&name);
            if !target.exists() {
                anyhow::bail!("Workspace '{}' does not exist.", name);
            }

            // Prevent deleting the currently active workspace
            let config = runtime.load_config()?;
            let current_workspace = runtime.effective_workspace(&config);
            let canonical_current = std::fs::canonicalize(&current_workspace).ok();
            let canonical_target = std::fs::canonicalize(&target).ok();
            if canonical_current.is_some() && canonical_current == canonical_target {
                anyhow::bail!("Cannot delete the currently active workspace. Switch to another workspace first.");
            }

            if !force {
                let confirm = dialoguer::Confirm::new()
                    .with_prompt(format!("Are you sure you want to delete workspace '{}'?", name))
                    .default(false)
                    .interact()?;
                if !confirm {
                    println!("Deletion cancelled.");
                    return Ok(());
                }
            }

            std::fs::remove_dir_all(&target)?;
            println!("Deleted workspace '{}'.", name);
        }
    }

    Ok(())
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
