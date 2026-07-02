# Summary

- This iteration finishes the next step of slice 3 for the manager control plane layering work.
- Provider companion logic in `agent-diva-manager` was moved out of the mixed admin implementation into a dedicated manager module and a dedicated handler module.
- `agent-diva-manager/src/manager/provider_admin.rs` now owns provider-specific manager actions.
- `agent-diva-manager/src/handlers/provider_companion.rs` now owns provider-specific HTTP handlers.
- Existing command routing stays the same:
  - `ManagerCommand::Provider(ProviderCommand)`
  - `manager.rs` still dispatches through `handle_provider_command(...)`
  - `server.rs` still exposes the same `/api/providers*` routes

# Impact

- Internal boundaries are clearer between runtime control and provider companion control.
- `companion_admin.rs` is reduced to skill and MCP concerns instead of mixing provider logic into the same file.
- `handlers.rs` keeps the same public exports for server wiring, so external route shape and frontend call sites remain unchanged.
- This is a structural cleanup only. No intended behavior change was introduced in this iteration.
