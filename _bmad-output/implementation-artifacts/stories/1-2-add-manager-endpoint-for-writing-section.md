---
baseline_commit: 68f37ca16e91aac0cffe421b17d20820047dcd9b
status: review
---

# Story 1.2: Add manager endpoint for writing a section

Status: review

## Story

As a developer,
I want a `POST /api/laputa/section/:name/write` endpoint in the manager,
So that the GUI can request auditable section writes over HTTP.

## Acceptance Criteria

1. **Given** the manager is running with a configured `LaputaService`,
   **When** a client sends `POST /api/laputa/section/:name/write` with `{ "content": "...", "actor": "gui-user", "summary": "optional" }`,
   **Then** the handler calls `LaputaService::create_and_apply_direct_edit(section, content, actor, Utc::now())` and returns `{ "status": "ok", "changelog_id": "...", "applied_at": "..." }`.

2. **Given** a request with an unknown section name,
   **When** the handler parses the `:name` path parameter,
   **Then** it returns `404 Not Found` with `code: "unknown_section"` and a clear message.

3. **Given** a request with a malformed JSON body,
   **When** axum tries to deserialize the payload,
   **Then** it returns `400 Bad Request` with a clear error message.

4. **Given** a request with invalid content for the target section (e.g. non-JSON patch on a JSON-only section),
   **When** `create_and_apply_direct_edit` returns `LaputaError::SchemaIncompatible` or `LaputaError::UnauthorizedTarget`,
   **Then** the handler returns `422 Unprocessable Entity` with the Laputa error code and message.

5. **Given** the new handler is implemented,
   **When** `agent-diva-manager/src/server.rs` is inspected,
   **Then** `laputa_routes()` registers `POST /api/laputa/section/:name/write` pointing to the new handler.

6. **Given** the manager crate tests are run,
   **When** `cargo test -p agent-diva-manager` executes,
   **Then** the new integration tests for the write endpoint pass and existing manager tests remain green.

## Tasks / Subtasks

- [x] **Add request DTO in `agent-diva-manager/src/handlers/laputa.rs`** (AC: #1)
  - [x] Open `agent-diva-manager/src/handlers/laputa.rs`
  - [x] Add a new payload struct:
        ```rust
        #[derive(Debug, Deserialize)]
        pub struct WriteLaputaSectionPayload {
            pub content: String,
            pub actor: Option<String>,
            pub summary: Option<String>,
        }
        ```
  - [x] The `summary` field is accepted for forward compatibility; it may be ignored by the first implementation or included in proposal evidence if `LaputaService` supports it.

- [x] **Implement `write_laputa_section_handler`** (AC: #1, #2, #4)
  - [x] Add a new async handler function in `agent-diva-manager/src/handlers/laputa.rs`:
        ```rust
        pub async fn write_laputa_section_handler(
            State(state): State<AppState>,
            Path(name): Path<String>,
            Json(payload): Json<WriteLaputaSectionPayload>,
        ) -> JsonResult {
            let section = LaputaSectionName::from_str(&name).map_err(|_| {
                error_response(
                    StatusCode::NOT_FOUND,
                    "unknown_section",
                    format!("unknown Laputa section: {name}"),
                )
            })?;

            let outcome = state
                .laputa
                .create_and_apply_direct_edit(
                    section,
                    payload.content,
                    payload.actor.unwrap_or_else(|| "api".to_string()),
                    Utc::now(),
                )
                .map_err(laputa_error_response)?;

            ok(serde_json::json!({
                "status": "ok",
                "changelog_id": outcome.changelog.id,
                "applied_at": outcome.changelog.created_at,
            }))
        }
        ```
  - [x] Reuse the existing `JsonResult`, `ok`, `error_response`, and `laputa_error_response` helpers already defined in the module.
  - [x] Use the same `LaputaSectionName::from_str` parsing and `404` mapping pattern as `get_laputa_section_handler`.

- [x] **Register the route in `agent-diva-manager/src/server.rs`** (AC: #5)
  - [x] Open `agent-diva-manager/src/server.rs`
  - [x] Import the new handler at the top of the file:
        ```rust
        use crate::handlers::{
            // ... existing imports ...
            write_laputa_section_handler,
        };
        ```
  - [x] In `laputa_routes()`, add:
        ```rust
        .route(
            "/api/laputa/section/:name/write",
            post(write_laputa_section_handler),
        )
        ```
  - [x] Place the new route immediately after the existing `GET /api/laputa/section/:name` line to keep section routes grouped.

- [x] **Add manager-level integration tests** (AC: #1, #2, #3, #4, #6)
  - [x] Open `agent-diva-manager/src/server.rs` (existing `#[cfg(test)]` module at the bottom)
  - [x] Add tests using the existing `AppState::new(..., temp.path())` helper and `tower::ServiceExt::oneshot` pattern.
  - [x] Test successful write:
    - Build router with a temp workspace.
    - POST `{"content":"{\"note\":\"hello\"}"}` to `/api/laputa/section/memory_md/write`.
    - Assert status `200` and response contains non-empty `changelog_id` and `applied_at`.
    - Optionally verify the section content was updated via `state.laputa.read_section(...)`.
  - [x] Test unknown section:
    - POST to `/api/laputa/section/not_a_section/write`.
    - Assert status `404` and `code == "unknown_section"`.
  - [x] Test malformed JSON:
    - POST invalid JSON body.
    - Assert status `400`.
  - [x] Test schema-incompatible payload:
    - POST non-JSON content to a JSON-only section like `memory_md`.
    - Assert status `422` and `code == "schema_incompatible"`.

- [x] **Run validation gates** (AC: #6)
  - [x] `cargo test -p agent-diva-manager`
  - [x] `just fmt-check`
  - [x] `just check`
  - [x] Optional manual smoke test: start the manager and `curl` the endpoint.

## Dev Notes

### Relevant architecture patterns and constraints

- **All file writes must stay inside `agent-diva-laputa/src`**. The manager handler is only a thin HTTP adapter; it must not perform any direct filesystem writes. It extracts `AppState`, calls `state.laputa.create_and_apply_direct_edit(...)`, and maps the result. This keeps the `authority_boundary_guard` tests passing.
- **Governance flow is enforced by `LaputaService`**. Story 1.1 adds `create_and_apply_direct_edit`, which internally creates an `EvolutionProposal`, transitions it to `Approved`, and applies it. This story only exposes that method over HTTP.
- **Route registration follows existing axum patterns**. See `laputa_routes()` in `agent-diva-manager/src/server.rs`: section routes use `Path(name)` and live next to `/api/laputa/section/:name`.
- **Error mapping is centralized**. The module already defines `laputa_error_response`, which maps `LaputaError` variants to `404` / `422` / `500` status codes. Reuse it for all Laputa errors. Use `error_response(...)` for section-name parsing failures.
- **`JsonResult` type alias**. All handlers in this file return `type JsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;` so the new handler should conform to it.

### Source tree components to touch

- `agent-diva-manager/src/handlers/laputa.rs` — add `WriteLaputaSectionPayload` and `write_laputa_section_handler`
- `agent-diva-manager/src/server.rs` — import handler and register the route
- `agent-diva-manager/src/state.rs` — no changes expected; `AppState` already exposes `pub laputa: LaputaService`

### Request/response DTO suggestions

Request body:
```json
{
  "content": "# Identity\n\nI am...",
  "actor": "gui-user",
  "summary": "manual edit from Persona & Memory page"
}
```

Response body (success):
```json
{
  "status": "ok",
  "changelog_id": "chg-...",
  "applied_at": "2026-07-05T12:34:56Z"
}
```

Response body (error):
```json
{
  "status": "error",
  "code": "schema_incompatible",
  "message": "..."
}
```

### Testing standards summary

- Use `tempfile::tempdir()` and `AppState::new(api_tx, MessageBus::new(), temp.path())` to build an isolated manager state.
- Use `build_router(state).oneshot(...)` from `tower::ServiceExt` for lightweight HTTP-level tests.
- Include tests for success, unknown section, malformed JSON, and schema-incompatible payloads.
- Run `cargo test -p agent-diva-manager` and the workspace lint gates before claiming done.

### Project Structure Notes

- This story builds on Story 1.1 (`create_and_apply_direct_edit` in `agent-diva-laputa`). The manager endpoint cannot be fully implemented until that method exists, but the route, DTO, and handler skeleton can be drafted.
- The next story (1.3) will add the Tauri command that calls this manager endpoint.
- Do not write to `.laputa/sections` directly from the manager crate; any direct `fs::write` will fail the `authority_boundary_guard` test in `agent-diva-laputa/tests`.

### References

- Existing handler patterns: [Source: agent-diva-manager/src/handlers/laputa.rs]
- Route registration pattern: [Source: agent-diva-manager/src/server.rs]
- `AppState` and `LaputaService` wiring: [Source: agent-diva-manager/src/state.rs]
- `create_and_apply_direct_edit` specification: [Source: _bmad-output/implementation-artifacts/stories/1-1-add-direct-edit-and-apply-to-laputa-service.md]
- `ApplyOutcome` / `ChangelogRecord` types: [Source: agent-diva-laputa/src/proposals.rs] and [Source: agent-diva-core/src/evolution/types.rs]
- Architecture spine: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md]
- Epic source: [Source: _bmad-output/planning-artifacts/epics.md]

## Dev Agent Record

### Agent Model Used

kimi-for-coding

### Debug Log References

- `cargo test -p agent-diva-manager`: 56 passed (including 4 new write-endpoint tests)
- `just fmt-check`: clean
- `just check`: clean

### Completion Notes List

- [x] `WriteLaputaSectionPayload` added to `agent-diva-manager/src/handlers/laputa.rs`
- [x] `write_laputa_section_handler` implemented and returns `{ changelog_id, applied_at }`
- [x] Route `POST /api/laputa/section/:name/write` registered in `agent-diva-manager/src/server.rs`
- [x] Manager integration tests added and passing
- [x] `cargo test -p agent-diva-manager` passes
- [x] `just fmt-check && just check` clean

### File List

- `agent-diva-manager/src/handlers/laputa.rs`
- `agent-diva-manager/src/handlers.rs`
- `agent-diva-manager/src/server.rs`

### Change Log

- 2026-07-05: Implemented `POST /api/laputa/section/:name/write` handler, registered route, and added manager integration tests. Validation gates passed.
