//! Migration tool for agent-diva
//!
//! This tool helps migrate configuration and sessions from the Python version.

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::path::PathBuf;
// use tracing::{info, warn};

mod config_migration;
mod memory_migration;
mod session_migration;

use config_migration::ConfigMigrator;
use memory_migration::MemoryMigrator;
use session_migration::SessionMigrator;

#[derive(Parser)]
#[command(name = "agent-diva-migrate")]
#[command(about = "Migration tool for agent-diva - migrate from Python to Rust version")]
#[command(version)]
struct Cli {
    /// Perform a dry run without making changes
    #[arg(long)]
    dry_run: bool,

    /// Path to the old Python agent-diva config directory (default: ~/.agent-diva)
    #[arg(short, long)]
    source: Option<PathBuf>,

    /// Path to the new Rust agent-diva config directory (default: ~/.agent-diva)
    #[arg(short, long)]
    target: Option<PathBuf>,

    /// Skip config migration
    #[arg(long)]
    skip_config: bool,

    /// Skip sessions migration
    #[arg(long)]
    skip_sessions: bool,

    /// Skip memory migration
    #[arg(long)]
    skip_memory: bool,

    /// Auto-confirm all prompts
    #[arg(short, long)]
    yes: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    print_banner();

    if cli.dry_run {
        println!(
            "{}",
            style("Running in DRY-RUN mode (no changes will be made)").yellow()
        );
        println!();
    }

    // Determine source and target paths
    let source_dir = cli.source.unwrap_or_else(get_default_agent_diva_dir);
    let target_dir = cli.target.unwrap_or_else(get_default_agent_diva_dir);

    println!("Source (Python): {}", style(source_dir.display()).cyan());
    println!("Target (Rust):   {}", style(target_dir.display()).cyan());
    println!();

    // Confirm migration
    if !cli.yes && !cli.dry_run && !confirm("Do you want to proceed with the migration?")? {
        println!("Migration cancelled.");
        return Ok(());
    }

    let mut migrated = false;

    // Migrate configuration
    if !cli.skip_config {
        let config_migrator = ConfigMigrator::new(&source_dir, &target_dir);
        match config_migrator.migrate(cli.dry_run).await {
            Ok(result) => {
                print_config_result(&result);
                migrated = true;
            }
            Err(e) => {
                eprintln!("{} Config migration failed: {}", style("✗").red(), e);
            }
        }
    }

    // Migrate sessions
    if !cli.skip_sessions {
        let session_migrator = SessionMigrator::new(&source_dir, &target_dir);
        match session_migrator.migrate(cli.dry_run).await {
            Ok(result) => {
                print_session_result(&result);
                migrated = true;
            }
            Err(e) => {
                eprintln!("{} Sessions migration failed: {}", style("✗").red(), e);
            }
        }
    }

    // Migrate memory
    if !cli.skip_memory {
        let memory_migrator = MemoryMigrator::new(&source_dir, &target_dir);
        match memory_migrator.migrate(cli.dry_run).await {
            Ok(result) => {
                print_memory_result(&result);
                migrated = true;
            }
            Err(e) => {
                eprintln!("{} Memory migration failed: {}", style("✗").red(), e);
            }
        }
    }

    println!();
    if migrated {
        println!("{}", style("Migration completed!").green().bold());
        if cli.dry_run {
            println!("Run without --dry-run to apply the changes.");
        }
    } else {
        println!("{}", style("Nothing to migrate.").yellow());
    }

    Ok(())
}

fn print_banner() {
    println!();
    println!(
        "{}",
        style("╔═══════════════════════════════════════════╗").cyan()
    );
    println!(
        "{}",
        style("║      Agent Diva Migration Tool            ║").cyan()
    );
    println!(
        "{}",
        style("║      Python → Rust Version                ║").cyan()
    );
    println!(
        "{}",
        style("╚═══════════════════════════════════════════╝").cyan()
    );
    println!();
}

fn get_default_agent_diva_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h: PathBuf| h.join(".agent-diva"))
        .unwrap_or_else(|| PathBuf::from(".agent-diva"))
}

fn confirm(prompt: &str) -> Result<bool> {
    use dialoguer::Confirm;

    Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .context("Failed to read user input")
}

fn print_config_result(result: &config_migration::MigrationResult) {
    println!();
    println!("{}", style("Configuration Migration").bold());
    println!("{}", style("─────────────────────").dim());

    if result.migrated {
        println!("{} Config file migrated successfully", style("✓").green());
        println!("  Source: {}", style(result.source_path.display()).dim());
        println!("  Target: {}", style(result.target_path.display()).dim());
    } else if result.already_exists {
        println!(
            "{} Config already exists at target, skipped",
            style("○").yellow()
        );
    } else {
        println!("{} No config found at source", style("○").dim());
    }
}

fn print_session_result(result: &session_migration::MigrationResult) {
    println!();
    println!("{}", style("Sessions Migration").bold());
    println!("{}", style("──────────────────").dim());

    if result.total > 0 {
        println!(
            "{} Migrated {}/{} sessions",
            style("✓").green(),
            style(result.successful).green(),
            style(result.total).cyan()
        );
        if result.failed > 0 {
            println!("  {} {} failed", style("✗").red(), result.failed);
        }
    } else {
        println!("{} No sessions found to migrate", style("○").dim());
    }
}

fn print_memory_result(result: &memory_migration::MigrationResult) {
    println!();
    println!("{}", style("Memory Migration").bold());
    println!("{}", style("────────────────").dim());

    if result.total > 0 {
        println!(
            "{} Migrated {}/{} memory files",
            style("✓").green(),
            style(result.successful).green(),
            style(result.total).cyan()
        );
        if result.failed > 0 {
            println!("  {} {} failed", style("✗").red(), result.failed);
        }
    } else {
        println!("{} No memory files found to migrate", style("○").dim());
    }
}
