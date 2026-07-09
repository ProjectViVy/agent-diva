use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Component, Path, PathBuf};

use crate::cli_runtime::CliRuntime;
use agent_diva_core::config::Config;

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum WorkspaceCommands {
    /// List managed workspaces under config-dir/workspaces
    List,
    /// Create a new managed workspace under config-dir/workspaces
    Create {
        /// Workspace name
        name: String,
        /// Optional source path to copy/symlink from
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Switch config.json to a managed workspace by name
    Switch {
        /// Workspace name
        name: String,
    },
    /// Delete a managed workspace by name
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

    match command {
        WorkspaceCommands::List => {
            if !workspaces_root.exists() {
                println!("No managed workspaces found.");
                return Ok(());
            }

            let entries = std::fs::read_dir(&workspaces_root).with_context(|| {
                format!(
                    "Failed to read workspaces dir: {}",
                    workspaces_root.display()
                )
            })?;

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
                println!("No managed workspaces found.");
            } else {
                names.sort();
                println!("Managed workspaces:");
                for name in names {
                    println!("  {}", name);
                }
            }
        }
        WorkspaceCommands::Create { name, path } => {
            std::fs::create_dir_all(&workspaces_root)?;
            let target = resolve_workspace_target(&workspaces_root, &name)?;
            if target.exists() {
                anyhow::bail!("Workspace '{}' already exists.", name);
            }

            if let Some(src) = path {
                // Copy directory contents from source
                copy_dir_all(&src, &target)?;
                println!(
                    "Created workspace '{}' from {} (managed under config-dir/workspaces)",
                    name,
                    src.display()
                );
            } else {
                std::fs::create_dir_all(&target)?;
                println!(
                    "Created workspace '{}' at {} (managed under config-dir/workspaces)",
                    name,
                    target.display()
                );
            }
        }
        WorkspaceCommands::Switch { name } => {
            let target = resolve_workspace_target(&workspaces_root, &name)?;
            if !target.exists() {
                anyhow::bail!("Workspace '{}' does not exist.", name);
            }

            let mut config: Config = runtime.load_config()?;
            let workspace_path = target.display().to_string();
            config.agents.defaults.workspace = workspace_path;
            runtime.loader().save(&config)?;

            println!(
                "Switched to workspace '{}' (managed under config-dir/workspaces).",
                name
            );
            println!("Restart gateway for changes to take effect.");
        }
        WorkspaceCommands::Delete { name, force } => {
            let target = resolve_workspace_target(&workspaces_root, &name)?;
            if !target.exists() {
                anyhow::bail!("Workspace '{}' does not exist.", name);
            }

            // Prevent deleting the currently active workspace
            let config = runtime.load_config()?;
            let persisted_workspace = expand_config_workspace(&config);
            let canonical_current = std::fs::canonicalize(&persisted_workspace).ok();
            let canonical_target = std::fs::canonicalize(&target).ok();
            if canonical_current.is_some() && canonical_current == canonical_target {
                anyhow::bail!("Cannot delete the currently active workspace. Switch to another workspace first.");
            }

            if !force {
                let confirm = dialoguer::Confirm::new()
                    .with_prompt(format!(
                        "Are you sure you want to delete workspace '{}'?",
                        name
                    ))
                    .default(false)
                    .interact()?;
                if !confirm {
                    println!("Deletion cancelled.");
                    return Ok(());
                }
            }

            std::fs::remove_dir_all(&target)?;
            println!(
                "Deleted workspace '{}' (managed under config-dir/workspaces).",
                name
            );
        }
    }

    Ok(())
}

fn resolve_workspace_target(workspaces_root: &Path, name: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        is_valid_workspace_name(name),
        "Invalid workspace name '{}'. Use a single directory name without path separators or traversal segments.",
        name
    );

    Ok(workspaces_root.join(name))
}

fn is_valid_workspace_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    matches!(
        Path::new(name).components().next(),
        Some(Component::Normal(_))
    ) && Path::new(name).components().count() == 1
}

fn expand_config_workspace(config: &Config) -> PathBuf {
    let workspace = &config.agents.defaults.workspace;
    if workspace == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else if let Some(rest) = workspace.strip_prefix("~/") {
        dirs::home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(workspace))
    } else {
        PathBuf::from(workspace)
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
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
