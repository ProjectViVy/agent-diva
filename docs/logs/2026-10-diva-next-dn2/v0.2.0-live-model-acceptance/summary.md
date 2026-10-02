# DN-2 task 5: packaged live-model acceptance (v0.2.0)

Native end-to-end run of the sealed DIVA shared library
(`src-tauri/vivy-runtime/vivy-shared.so`, recipe `diva.vivy.yml`)
driven through the real C ABI by `drive.py` (python ctypes, no mocks).

## Environment

- linux/amd64, temp workspace `~/diva-dn2-accept/` (config + sqlite +
  workspace root all inside; no production data).
- Provider: `sensenova` (embedded vendor), model
  `sensenova-6.8-flash-lite`, OpenAI-compatible endpoint
  `https://token.sensenova.cn/v1`. Key supplied via
  `SENSENOVA_API_KEY` (D-010 env-only); no key appears in evidence.
- Policy: default `workspace_write` + `approval_policy: ask`;
  `write_file` is effectful (not auto-approved).

## What the transcript proves (16/16 checks)

1. `VivyInit(config_path)` → `initialize` → `session/create` →
   `turn/start` → `run/subscribe` → streamed `model.reasoning_delta` /
   `model.delta` / `model.usage` → `run.completed`; Journal readback
   (`session/messages`) persisted the answer.
2. Tool gate: `tool.requested(write_file)` → `policy.evaluated
   (decision=prompt)` → `tool.approval_required(apr_*)` — file absent
   before any decision.
3. `review/respond {action: deny}` → run completed, file still absent.
4. Second turn → new `tool.approval_required` → `review/respond
   {action: approve}` → file landed at `workspace/<run_id>/hello.txt`
   with the expected content (sandbox world = one private dir per run).
5. `run/cancel` on an in-flight essay turn → `run.cancelled` terminal;
   `run/get` reports `status: cancelled`.
6. `review/list` returns both decisions settled (denied + approved) —
   snapshot authority reconstructs pending/settled interactions without
   resending anything.
7. `session/delete` + `VivyShutdown` clean teardown.

Evidence: `transcript.jsonl` (116 events + calls + checks),
`drive.out`, `drive.py`.
