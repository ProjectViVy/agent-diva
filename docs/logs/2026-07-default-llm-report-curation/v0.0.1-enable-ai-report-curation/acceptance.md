# Acceptance

1. Use a config with a valid default provider and no `reports.llm_curation` override.
2. Regenerate a daily, weekly, or monthly Notebook report.
3. Confirm report frontmatter contains `generation_mode: llm_curated`.
4. Set `reports.llm_curation.enabled=false`, regenerate, and confirm deterministic fallback remains available.
