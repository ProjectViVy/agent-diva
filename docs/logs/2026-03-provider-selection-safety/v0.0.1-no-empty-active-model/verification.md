# Verification

- `pnpm build` (workdir: `agent-diva-gui`)
- `cargo check -p agent-diva-gui`

# Results

- `pnpm build`: passed
- `cargo check -p agent-diva-gui`: passed

# Notes

- This fix is scoped to the GUI-side provider selection state handling that was writing an empty active model into config.
