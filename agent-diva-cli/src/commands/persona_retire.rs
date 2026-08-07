//! Persona retirement commands (LAPUTA-COGNITIVE-SYNC S4).
//!
//! One-shot migration flow for retiring the workspace persona file layer:
//! `plan` (dry run) → `propose` (Frozen Core proposals, human review) →
//! approve/apply through `approvals` → `archive` (inert `.laputa/legacy/`).

use anyhow::Result;
use clap::Subcommand;

use crate::cli_runtime::CliRuntime;

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PersonaRetireCommands {
    /// Show what would be retired (dry run, no writes)
    Plan,
    /// Create Frozen Core migration proposals pending human review
    Propose,
    /// Move migrated sources into .laputa/legacy/ (after approval and apply)
    Archive,
}

pub async fn run(command: PersonaRetireCommands, runtime: &CliRuntime) -> Result<()> {
    let config = runtime.load_config()?;
    let workspace = runtime.effective_workspace(&config);
    let plan = agent_diva_laputa::scan_persona_workspace(&workspace)
        .map_err(|error| anyhow::anyhow!("persona retirement scan failed: {error}"))?;

    if plan.is_empty() {
        println!("No retired persona files found in {}.", workspace.display());
        return Ok(());
    }

    match command {
        PersonaRetireCommands::Plan => {
            for spec in &plan.proposals {
                let sources = spec
                    .sources
                    .iter()
                    .map(|source| source.path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "proposal: {} -> {} ({sources})",
                    sources,
                    spec.target_section.as_str()
                );
            }
            for archive in &plan.archives {
                println!("archive-only: {}", archive.display());
            }
            println!("Run 'persona-retire propose' to create review proposals.");
        }
        PersonaRetireCommands::Propose => {
            let service = agent_diva_laputa::LaputaService::open(&workspace)
                .map_err(|error| anyhow::anyhow!("laputa service unavailable: {error}"))?;
            let proposals = agent_diva_laputa::create_persona_retirement_proposals(
                &service,
                &plan,
                "cli:persona-retire",
                chrono::Utc::now(),
            )
            .map_err(|error| anyhow::anyhow!("proposal creation failed: {error}"))?;
            for proposal in &proposals {
                println!(
                    "created proposal {} -> {} (pending review)",
                    proposal.id,
                    proposal.target_section.as_str()
                );
            }
            for archive in &plan.archives {
                println!("archive-only (no proposal): {}", archive.display());
            }
            println!("Approve and apply via 'approvals', then run 'persona-retire archive'.");
        }
        PersonaRetireCommands::Archive => {
            if !plan.proposals.is_empty() {
                let service = agent_diva_laputa::LaputaService::open(&workspace)
                    .map_err(|error| anyhow::anyhow!("laputa service unavailable: {error}"))?;
                for spec in &plan.proposals {
                    let section =
                        service
                            .read_section(spec.target_section.clone())
                            .map_err(|error| {
                                anyhow::anyhow!(
                                    "section {} unreadable: {error}",
                                    spec.target_section.as_str()
                                )
                            })?;
                    let migrated = section.content["metadata"]["migration"]
                        .as_str()
                        .is_some_and(|value| value == "persona_retirement");
                    if !migrated {
                        anyhow::bail!(
                            "section {} does not contain an applied persona retirement migration; approve and apply the proposals first",
                            spec.target_section.as_str()
                        );
                    }
                }
            }
            let outcome = agent_diva_laputa::archive_persona_sources(&workspace, &plan)
                .map_err(|error| anyhow::anyhow!("archive failed: {error}"))?;
            for (source, target) in &outcome.moved {
                println!("archived {} -> {}", source.display(), target.display());
            }
            for kept in &outcome.kept {
                println!(
                    "kept {} (archive target already exists; resolve manually)",
                    kept.display()
                );
            }
            if outcome.moved.is_empty() && outcome.kept.is_empty() {
                println!("Nothing left to archive.");
            }
        }
    }
    Ok(())
}
