# 完成归档：2026-08-23 TODOLIST 自动收尾批次

2026-08-23 用户确认：根清单 Done 区已关闭条目可以归档。本文件收录当日机械收尾、
合同冻结、用户确认真机/不可复现关闭的完成项。归档规则同 [`README.md`](README.md)：
保留原始上下文，不再驱动实施或验收；恢复须先重新验证问题仍存在，再以新活跃条目
写回根清单。

关闭批次合入本地 `dev`（未推送），工作树 `chore/todolist-auto-close`。

## 机械修复 / 合同冻结

- [x] **BML-GOVERNED-SEAM-DEAD-CODE** `sev-P3`
  Closed 2026-08-23 on `chore/bml-governed-seam-dead-code`: deleted
  `TypedMemoryStore::put_governed` / `rollback_governed` and the
  `put_inner` governed branch; kept `memory_apply_journal` DDL and
  `SCHEMA_VERSION = 1`. `bml_boundary_guard` still scans the retired
  names. Logs:
  [`docs/logs/2026-08-bml-governed-seam-dead-code/v0.1.0-remove-governed-store-apis/`](../../../../../../../logs/2026-08-bml-governed-seam-dead-code/v0.1.0-remove-governed-store-apis/summary.md).

- [x] **MEMORY-CRUD-PROPOSAL-CREATED-DEAD-ENUM** `sev-P3`
  Closed 2026-08-23 on `chore/memory-crud-proposal-created-dead-enum`: deleted
  `MemoryCrudOutcome::ProposalCreated` and `SyncTurnStatus::ProposalCreated`,
  consolidation dead match arms, and the lock-old proposal provider test.
  Logs:
  [`docs/logs/2026-08-memory-crud-proposal-created/v0.1.0-remove-proposal-created-enum/`](../../../../../../../logs/2026-08-memory-crud-proposal-created/v0.1.0-remove-proposal-created-enum/summary.md).

- [x] **LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY** `sev-P3`
  Closed 2026-08-23 on `chore/todolist-auto-close`: moved shared integration
  scanners to `tests/common/` so `--all-targets` clippy no longer flags
  per-binary dead code. Production lib and `just check` were already clean.
  Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.1.0-laputa-all-targets-clippy/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.1.0-laputa-all-targets-clippy/summary.md).

- [x] **SANDBOX-WINDOWS-RESTRICTED-TOKEN-ENV** `sev-P2`
  Closed 2026-08-23 on `chore/todolist-auto-close`: Restricted Token tests skip
  when `is_available()` is false, and still assert when the token can be
  created. Production executor unchanged. Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.2.0-sandbox-restricted-token-skip/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.2.0-sandbox-restricted-token-skip/summary.md).

- [x] **MSRV-ISOLATED-TARGET-CACHE** `sev-P2`
  Closed 2026-08-23 on `chore/todolist-auto-close`: `just msrv-probe` sets
  `CARGO_TARGET_DIR=target/msrv-1.80` for every `cargo +1.80` probe. Does not
  resolve `WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS`. Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.3.0-msrv-isolated-target-cache/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.3.0-msrv-isolated-target-cache/summary.md).

- [x] **GUI-TAURI-PLAN-STREAM-DISCONNECT** `sev-P3`
  Closed 2026-08-23 on `chore/todolist-auto-close`: plan execution uses
  `send_message`'s `saw_terminal` fallback; no leftover plan-only EventSource
  loop. Background/approval reconnect loops left unchanged. Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.4.0-plan-sse-disconnect/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.4.0-plan-sse-disconnect/summary.md).

- [x] **PLAN-MODE-PHYSICAL-STATE-MACHINE：Plan Mode 物理限制状态机** `sev-P1`
  Closed 2026-08-23 on `chore/todolist-auto-close`: froze the existing
  fail-closed matrix as an independent acceptance record; added a
  `ToolStepPolicy` seam test. Did not rebuild permission mode. Logs:
  [`docs/logs/2026-08-plan-mode-physical-state-machine/v0.1.0-contract-freeze/`](../../../../../../../logs/2026-08-plan-mode-physical-state-machine/v0.1.0-contract-freeze/summary.md).

- [x] **LAPUTA-STORAGE-STALE-LOCK-FLAKE** `sev-P2`
  Closed 2026-08-23 on `chore/todolist-auto-close`: stale recovery is
  mtime-only; leftover `pid=` lock files can be reclaimed. Default
  `stale_after` remains 5 minutes. Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.5.0-stale-lock-recovery/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.5.0-stale-lock-recovery/summary.md).

- [x] **CLI-WIREMOCK-502-PREEXISTING** `sev-P2`
  Closed 2026-08-23 on `chore/todolist-auto-close`: loopback `ApiClient`
  uses `no_proxy()` so Windows HTTP_PROXY cannot 502 wiremock. Focused
  approval tests pass; does not claim full `just test`. Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.6.0-cli-wiremock-no-proxy/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.6.0-cli-wiremock-no-proxy/summary.md).

## 用户确认关闭

- [x] **SKILL-MARKETPLACE-V1-TOKEN-VERIFY：skills.sh v1 API token 路径验证** `sev-P3`
  Closed 2026-08-23（用户确认）：skill 获取链路实测可用，
  `fetch_marketplace_featured.py` 的 token / v1 API 路径验证完成。

- [x] **STEPFUN-REAL-ENDPOINT-E2E：StepFun model pass-through** `sev-P3`
  Closed 2026-08-23（用户确认真机）：桌面 `keys.txt` 真实 endpoint E2E 已过。
  单测仍覆盖 model 透传；密钥和未脱敏响应未入库。Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.8.0-stepfun-real-endpoint-e2e/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.8.0-stepfun-real-endpoint-e2e/summary.md).

- [x] **WORKSPACE-GUI-TOOLING-LOAD-FLAKES** `sev-P2`
  Closed 2026-08-23（用户决策：不可复现、复发再开）。2026-08-11 全量套件下
  `embedded_gateway_serves_health_endpoint` 与 `agent-diva-tooling --lib`
  各偶发一次，focused 重跑与随后 `just ci` 通过；未做共享资源/时序隔离。
  复发须重开条目并补专项隔离。Logs:
  [`docs/logs/2026-08-todolist-auto-close/v0.7.0-workspace-gui-tooling-load-flakes/`](../../../../../../../logs/2026-08-todolist-auto-close/v0.7.0-workspace-gui-tooling-load-flakes/summary.md).
