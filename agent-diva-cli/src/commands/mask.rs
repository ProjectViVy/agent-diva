use agent_diva_agent::mask::{MaskError, MaskFile, MaskRegistry};
use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

use crate::cli_runtime::CliRuntime;

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MaskCommands {
    /// List all masks including the default identity
    List,
    /// Activate a mask by name or id
    Switch {
        /// Mask display name
        #[arg(short, long)]
        name: Option<String>,
        /// Stable mask id (file slug)
        #[arg(short, long)]
        id: Option<String>,
    },
    /// Print mask frontmatter and body
    Show {
        /// Mask display name
        #[arg(short, long)]
        name: Option<String>,
        /// Stable mask id (file slug)
        #[arg(short, long)]
        id: Option<String>,
    },
    /// Create a new mask from a markdown file
    Create {
        /// Path to the markdown mask file
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Replace an existing mask from a markdown file
    Edit {
        /// Mask display name
        #[arg(short, long)]
        name: Option<String>,
        /// Stable mask id (file slug)
        #[arg(short, long)]
        id: Option<String>,
        /// Path to the new markdown mask file
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Delete a mask by name or id
    Delete {
        /// Mask display name
        #[arg(short, long)]
        name: Option<String>,
        /// Stable mask id (file slug)
        #[arg(short, long)]
        id: Option<String>,
    },
}

pub async fn run(command: MaskCommands, runtime: &CliRuntime) -> Result<()> {
    let config = runtime.load_config().unwrap_or_default();
    let workspace = runtime.effective_workspace(&config);
    let masks_dir = workspace.join("masks");

    match command {
        MaskCommands::List => list_masks(&masks_dir),
        MaskCommands::Switch { name, id } => switch_mask(&masks_dir, name, id),
        MaskCommands::Show { name, id } => show_mask(&masks_dir, name, id),
        MaskCommands::Create { file } => create_mask(&masks_dir, &file),
        MaskCommands::Edit { name, id, file } => edit_mask(&masks_dir, name, id, &file),
        MaskCommands::Delete { name, id } => delete_mask(&masks_dir, name, id),
    }
}

fn list_masks(masks_dir: &std::path::Path) -> Result<()> {
    let registry = MaskRegistry::new(masks_dir);
    let current = registry.current_mask().map(|m| m.frontmatter.name.as_str());
    let masks = registry.list();

    if masks.is_empty() {
        println!("No masks found.");
        return Ok(());
    }

    println!("{:<4} {:<20} {:<10} Description", "", "Name", "Icon");
    println!("{}", "-".repeat(80));
    for mask in masks {
        let active = current == Some(mask.frontmatter.name.as_str());
        let marker = if active { "[*]" } else { "   " };
        let icon = mask.frontmatter.icon.as_deref().unwrap_or("");
        let desc = mask.frontmatter.description.as_deref().unwrap_or("");
        println!(
            "{marker} {name:<20} {icon:<10} {desc}",
            name = mask.frontmatter.name
        );
    }
    Ok(())
}

fn switch_mask(
    masks_dir: &std::path::Path,
    name: Option<String>,
    id: Option<String>,
) -> Result<()> {
    let mut registry = MaskRegistry::new(masks_dir);

    let mask = resolve_for_mutation(&mut registry, name.as_deref(), id.as_deref())?;
    let target_name = mask.frontmatter.name.clone();
    let target_id = mask.frontmatter.id_or_slug();

    // Use id-based switching when possible so the active-mask file stores the stable id.
    let activated = registry.switch_to_by_id(&target_id).map_err(into_anyhow)?;
    println!("Switched to mask: {}", activated.frontmatter.name);

    // If the user provided a name and we resolved to a different stored name (e.g., via id),
    // still report the canonical name.
    if activated.frontmatter.name != target_name {
        println!("  (resolved from id: {target_id})");
    }
    Ok(())
}

fn show_mask(masks_dir: &std::path::Path, name: Option<String>, id: Option<String>) -> Result<()> {
    let registry = MaskRegistry::new(masks_dir);
    let mask = resolve_for_read(&registry, name.as_deref(), id.as_deref())?;

    println!("---");
    println!("{}", mask.serialize().trim_end());
    Ok(())
}

fn create_mask(masks_dir: &std::path::Path, file: &std::path::Path) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read mask file: {}", file.display()))?;
    let mut mask =
        MaskFile::parse_with_path(&content, &file.display().to_string()).map_err(into_anyhow)?;

    // Ensure a stable id exists before persisting.
    let id = mask.frontmatter.id_or_slug();
    if mask.frontmatter.id.is_none() {
        mask = mask.with_id(id.clone());
    }

    let mut registry = MaskRegistry::new(masks_dir);
    let path = registry.mask_file_path(&id);
    let created = registry.create_or_update(mask).map_err(into_anyhow)?;
    println!(
        "Created mask '{}' at {}",
        created.frontmatter.name,
        path.display()
    );
    Ok(())
}

fn edit_mask(
    masks_dir: &std::path::Path,
    name: Option<String>,
    id: Option<String>,
    file: &std::path::Path,
) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read mask file: {}", file.display()))?;
    let mut new_mask =
        MaskFile::parse_with_path(&content, &file.display().to_string()).map_err(into_anyhow)?;

    let mut registry = MaskRegistry::new(masks_dir);
    let existing = resolve_for_mutation(&mut registry, name.as_deref(), id.as_deref())?;
    let existing_id = existing.frontmatter.id_or_slug();

    // Preserve the existing mask's id so the file path stays stable even if the new
    // file omitted an id or used a different one.
    if new_mask.frontmatter.id != Some(existing_id.clone()) {
        new_mask = new_mask.with_id(existing_id.clone());
    }

    let updated = registry.create_or_update(new_mask).map_err(into_anyhow)?;
    println!("Updated mask '{}'", updated.frontmatter.name);
    Ok(())
}

fn delete_mask(
    masks_dir: &std::path::Path,
    name: Option<String>,
    id: Option<String>,
) -> Result<()> {
    let mut registry = MaskRegistry::new(masks_dir);
    let target = resolve_for_mutation(&mut registry, name.as_deref(), id.as_deref())?;
    let target_name = target.frontmatter.name.clone();

    registry.delete(&target_name).map_err(into_anyhow)?;
    println!("Deleted mask: {}", target_name);
    Ok(())
}

/// Resolve a mask for read-only display. If both `name` and `id` are omitted,
/// show the currently active mask (or default).
fn resolve_for_read<'a>(
    registry: &'a MaskRegistry,
    name: Option<&str>,
    id: Option<&str>,
) -> Result<&'a MaskFile> {
    if let Some(id) = id {
        return registry
            .get_by_id(id)
            .ok_or_else(|| anyhow::anyhow!("Mask not found with id: {}", id));
    }

    if let Some(name) = name {
        return resolve_by_name(registry, name);
    }

    registry
        .current_mask()
        .or_else(|| registry.get(MaskFile::DEFAULT_NAME))
        .ok_or_else(|| anyhow::anyhow!("No active mask and default mask is unavailable"))
}

/// Resolve a mask for a mutation command. Requires an explicit `name` or `id`.
fn resolve_for_mutation<'a>(
    registry: &'a mut MaskRegistry,
    name: Option<&str>,
    id: Option<&str>,
) -> Result<&'a MaskFile> {
    if let Some(id) = id {
        return registry
            .get_by_id(id)
            .ok_or_else(|| anyhow::anyhow!("Mask not found with id: {}", id));
    }

    if let Some(name) = name {
        return resolve_by_name(registry, name);
    }

    anyhow::bail!("Either --name or --id is required")
}

fn resolve_by_name<'a>(registry: &'a MaskRegistry, name: &str) -> Result<&'a MaskFile> {
    if name == MaskFile::DEFAULT_NAME {
        return registry
            .get(MaskFile::DEFAULT_NAME)
            .ok_or_else(|| anyhow::anyhow!("Default mask is unavailable"));
    }

    let matches: Vec<&MaskFile> = registry
        .list()
        .into_iter()
        .filter(|m| m.frontmatter.name == name || m.frontmatter.id_or_slug() == name)
        .collect();

    match matches.len() {
        0 => Err(anyhow::anyhow!("Mask not found: {}", name)),
        1 => Ok(matches[0]),
        _ => Err(anyhow::anyhow!(
            "Ambiguous mask name: \"{}\" — use --id to disambiguate",
            name
        )),
    }
}

fn into_anyhow(err: MaskError) -> anyhow::Error {
    anyhow::anyhow!("{}", err)
}
