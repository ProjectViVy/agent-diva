---
baseline_commit: e3c21b633c2c027c7397e76bbfff57beb92e539f
status: review
---

# Story 1.3: Add Tauri command for writing a section

Status: review

## Story

As a GUI developer,
I want a Tauri command `laputa_write_section`,
So that the frontend can invoke the new manager endpoint `POST /api/laputa/section/:name/write` through the Tauri bridge to save an audited section edit.

## Acceptance Criteria

1. **Given** the GUI is running in Tauri mode,
   **When** `invoke('laputa_write_section', { name, content, summary })` is called,
   **Then** the command proxies a `POST` request to `{api_base_url}/laputa/section/{name}/write` with the payload `{ content, actor, summary }` and returns the manager's response body.

2. **Given** the new command is implemented,
   **When** inspecting `agent-diva-gui/src-tauri/src/lib.rs`,
   **Then** `laputa_write_section` is registered in the `tauri::generate_handler!` macro alongside the existing Laputa commands.

3. **Given** the manager returns a non-2xx or `{ "status": "error" }` response,
   **When** the command receives it,
   **Then** it returns a structured `serde_json::Value` error to the frontend containing `status`, `http_status`, `code`, `message`, and the original `body`, without panicking or swallowing the detail.

4. **Given** the command is implemented,
   **When** it is reviewed against the authority boundary guard,
   **Then** it contains no direct filesystem writes (`fs::write`, `File::create`, `atomic_write`, etc.); all persistence is delegated to the manager HTTP API.

5. **Given** the command is implemented,
   **When** `agent-diva-gui` is built and `pnpm tauri dev` is started,
   **Then** the frontend can call `writeLaputaSection(name, content, summary?)` from `desktop.ts` and receive a changelog-shaped result on success.

## Tasks / Subtasks

- [x] **Add `laputa_write_section` to `commands.rs`** (AC: #1, #3, #4)
  - [x] Open `agent-diva-gui/src-tauri/src/commands.rs`
  - [x] Add a new Tauri command immediately after the existing Laputa commands (e.g. after `laputa_get_section` or `laputa_rollback_changelog`) to keep the Laputa command cluster together:
        ```rust
        #[tauri::command]
        pub async fn laputa_write_section(
            name: String,
            content: String,
            #[allow(non_snake_case)] summary: Option<String>,
            state: State<'_, AgentState>,
        ) -> Result<serde_json::Value, serde_json::Value> {
            let url = format!(
                "{}/laputa/section/{}/write",
                state.api_base_url(),
                urlencoding::encode(name.trim())
            );
            let payload = serde_json::json!({
                "content": content,
                "actor": "gui-user",
                "summary": summary,
            });
            post_laputa_full_response(&state, &url, &payload).await
        }
        ```
  - [x] Use `post_laputa_full_response` (already defined in the same file) so that the entire manager response body is forwarded to the frontend. This preserves any extra fields the manager may add in the future and avoids hard-coding a single extraction field.
  - [x] Keep the command async, accept `State<'_, AgentState>`, and reuse `state.client` indirectly through the existing helper.
  - [x] Ensure no filesystem imports or direct file operations are introduced.

- [x] **Register the command in `lib.rs`** (AC: #2)
  - [x] Open `agent-diva-gui/src-tauri/src/lib.rs`
  - [x] Add `commands::laputa_write_section` to the `tauri::generate_handler!` macro, grouped with the other Laputa commands (for example, right after `commands::laputa_get_section`):
        ```rust
        commands::laputa_get_snapshot,
        commands::laputa_get_section,
        commands::laputa_write_section,
        commands::laputa_list_proposals,
        ...
        ```

- [x] **Add the frontend wrapper in `desktop.ts`** (AC: #5)
  - [x] Open `agent-diva-gui/src/api/desktop.ts`
  - [x] Add a `WriteLaputaSectionResult` interface and `writeLaputaSection` helper in the Laputa / Evolution Governance API section, alongside `getLaputaSection`:
        ```ts
        export interface WriteLaputaSectionResult {
          changelog_id: string;
          applied_at: string;
          status?: string;
        }

        export const writeLaputaSection = (
          name: LaputaSectionName,
          content: string,
          summary?: string,
        ) =>
          invoke<WriteLaputaSectionResult>("laputa_write_section", {
            name,
            content,
            summary: summary ?? null,
          });
        ```
  - [x] Note: The return type mirrors the manager response from Story 1.2 (`{ status: "ok", changelog_id, applied_at }`). Coordinate with Story 1.4 if the interface is centralized there.

- [x] **Run validation gates** (AC: #3, #5)
  - [x] `cargo check -p agent-diva-gui` (or `just check`)
  - [x] `just fmt-check`
  - [ ] `cd agent-diva-gui && pnpm tauri dev`
  - [ ] From the browser/Tauri devtools console, test:
        ```js
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("laputa_write_section", {
          name: "identity",
          content: "# Identity\n\nUpdated from Tauri bridge test.",
          summary: "Tauri bridge smoke test",
        });
        ```
  - [ ] Verify that success returns a changelog-shaped JSON and that an invalid section name returns a structured error with `status: "error"`.

## Dev Notes

### Relevant architecture patterns and constraints

- **Tauri commands are HTTP bridges, not persistence layers**. All file-system writes must stay inside `agent-diva-laputa`. The Tauri command must only serialize the payload and call the manager endpoint. Do not add `std::fs` operations or invoke local Laputa helpers from the GUI crate.
- **Use the existing helper suite** (`post_laputa_full_response`, `post_laputa_payload`, `parse_laputa_response`) for consistent error shaping. `post_laputa_full_response` is preferred here because the manager's write endpoint returns a flat object (`{ changelog_id, applied_at }`) rather than a wrapper like `{ status: "ok", proposal: ... }`. If Story 1.2 changes the response wrapper, the full-response approach keeps the Tauri command stable.
- **Error shape is standardized**. `parse_laputa_response` returns:
  ```json
  {
    "status": "error",
    "http_status": 400,
    "code": "...",
    "message": "...",
    "body": { ...original manager body... }
  }
  ```
  The frontend should display `message` to the user and may log `body` for debugging.
- **Payload fields**. The manager endpoint expects `{ content, actor?, summary? }` (see `docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md` §4.2). Set a default `actor` of `"gui-user"` in the Tauri command so the frontend does not have to pass it; make `summary` optional because quick saves from the editor may omit it.

### Source tree components to touch

- `agent-diva-gui/src-tauri/src/commands.rs` — add `laputa_write_section`
- `agent-diva-gui/src-tauri/src/lib.rs` — register in `generate_handler!`
- `agent-diva-gui/src/api/desktop.ts` — add `writeLaputaSection` wrapper
- No changes to `agent-diva-laputa`, `agent-diva-manager`, or filesystem persistence in this story.

### Testing standards summary

- Run Rust-level checks before launching the GUI:
  - `cargo check -p agent-diva-gui --lib`
  - `just fmt-check`
- Use `pnpm tauri dev` for interactive smoke testing. In the Tauri devtools console, call `invoke('laputa_write_section', ...)` directly. In a Vue component, import `writeLaputaSection` from `desktop.ts`.
- Verify both success and error paths:
  - Success: response contains `changelog_id` (and optionally `applied_at`).
  - Error: invalid section name or missing content returns a `serde_json::Value` error with a human-readable `message`.
- Confirm that `agent-diva-laputa/tests/authority_boundary_guard.rs` still passes after this change (it should, because no direct writes are added in GUI crate).

### Project Structure Notes

- This story consumes the manager endpoint implemented in **Story 1.2**. If Story 1.2 is not yet merged, you can temporarily mock the manager endpoint with `wiremock` or test the Tauri command's error path only. Coordinate the exact response shape with Story 1.2 before finalizing the `desktop.ts` return type.
- This story feeds **Story 1.4** (expose wrappers in `desktop.ts`) and **Epic 3** (frontend save flow). The Tauri command itself must be backend-agnostic beyond the manager HTTP contract.

### References

- Existing Laputa GET/POST command patterns: [Source: agent-diva-gui/src-tauri/src/commands.rs — `laputa_get_section`, `laputa_create_proposal`, `laputa_apply_proposal`]
- `AgentState` / `api_base_url()` / `state.client` usage: [Source: agent-diva-gui/src-tauri/src/app_state.rs]
- `generate_handler!` registration: [Source: agent-diva-gui/src-tauri/src/lib.rs]
- Manager endpoint contract: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md §4.2]
- Frontend `desktop.ts` Laputa wrappers: [Source: agent-diva-gui/src/api/desktop.ts]
- Epic source: [Source: _bmad-output/planning-artifacts/epics.md §Story 1.3]
- PRD: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md §FR-106]

## Dev Agent Record

### Agent Model Used

kimi-for-coding

### Debug Log References

- `cargo check -p agent-diva-gui` passed.
- `pnpm vue-tsc --noEmit` passed (no output).
- `just fmt-check` passed.
- `just check` passed.

### Completion Notes List

- [x] `laputa_write_section` added to `agent-diva-gui/src-tauri/src/commands.rs`
- [x] Command registered in `agent-diva-gui/src-tauri/src/lib.rs` `generate_handler!`
- [x] `writeLaputaSection` wrapper added to `agent-diva-gui/src/api/desktop.ts`
- [x] `cargo check -p agent-diva-gui` passes
- [x] `just fmt-check` passes
- [x] `just check` passes
- [x] `pnpm vue-tsc --noEmit` passes
- [ ] Smoke test in `pnpm tauri dev` succeeds for both happy path and error path
- [x] `agent-diva-laputa/tests/authority_boundary_guard.rs` still passes (no GUI-crate direct writes introduced)

### File List

- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva-gui/src/api/desktop.ts`

### Change Log

- Added `laputa_write_section` Tauri command that proxies `POST {api_base_url}/laputa/section/{name}/write` with payload `{ content, actor: "gui-user", summary }` and returns the full manager response.
- Registered `commands::laputa_write_section` in `generate_handler!` alongside existing Laputa commands.
- Added `WriteLaputaSectionResult` interface and `writeLaputaSection(name, content, summary?)` wrapper in `desktop.ts`, using `null` for omitted optional `summary`.
- Validated with `cargo check -p agent-diva-gui`, `just fmt-check`, `just check`, and `pnpm vue-tsc --noEmit`.
