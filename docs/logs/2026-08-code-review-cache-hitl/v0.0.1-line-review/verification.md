# Verification — 审查过程与证据

## 基线

```
HEAD     b4d2a84b450523152496ed7d064df007bb6646d4
branch   agent-diva-pro
ahead    61 (origin/agent-diva-pro)
time     2026-08-12
```

## 祖先检查（交付真相）

| Commit | 主题 | `merge-base --is-ancestor HEAD` |
|--------|------|----------------------------------|
| `2eeb5638` / `741b7ac3` | C1 cache | ON HEAD |
| `b6959dc2` / `02fe040c` / `ffff0fe6` | C3 artifact | ON HEAD |
| `66fb1ed4` / `8b79fcac` / `3ce9eba2` | C4/C5e deferred | ON HEAD |
| `73dff1bc` / `19a5bfec` / `e25a97fd` | C5a/b/d | ON HEAD |
| `956bdd66` / `cf268e5e` | ask_user | ON HEAD |
| `014e4347`…`bf6f6e8d` | 2026-07 HITL spine | **NOT**（`refactor/deep-governance`） |
| `2c8db02a`…`defb84fd` | M3 HITL S1–S5 | **NOT**（`feat/m3-hitl-closure`） |

## 符号 / 删除证明

```text
rg mount_tool|tool_discovery_v1|tool_not_discovered --glob '*.rs'  → 0 matches
rg command-approval-requested agent-diva-gui → 仅 Tauri emit，无 Vue listen
rg ApprovalCenter|ApprovalBanner ChatView.vue → 无内联审批卡
```

## L1 精读文件（节选）

| 文件 | 结论锚点 |
|------|----------|
| `agent-diva-core/src/tool_artifact/mod.rs` | 阈值、隔离、GC、显式失败 |
| `agent-diva-agent/src/tool_results.rs` | canonicalize + microcompact |
| `agent-diva-agent/src/agent_loop/turn/tool_step.rs` | 成功/错误 canonical 分支 |
| `agent-diva-agent/src/agent_loop/turn/iteration.rs` | microcompact 时序、budget |
| `agent-diva-tooling/src/registry.rs` | ActiveDeferred 8、not_active |
| `agent-diva-tools/src/tool_discovery.rs` | search 自动 activate |
| `agent-diva-providers/src/final_wire.rs` | core 终点 = 首 cache_control |
| `agent-diva-providers/src/openai_compatible.rs` | apply_cache_control / supports |
| `agent-diva-agent/src/compaction/compaction_exec.rs` | safe end + fold |
| `agent-diva-sandbox/src/guardian.rs` | HEAD 三模式合并臂 |
| `agent-diva-tools/src/shell.rs` | 无 Guardian 接线 |
| `agent-diva-gui/src/App.vue` | approval-event only + dual start |
| `agent-diva-gui/src/components/ChatView.vue` | permissionMode、tool row、ask_user |
| `agent-diva-tools/src/ask_user.rs` + core ask_user | timeout 600 |

## 侧分支对照

```text
git show feat/m3-hitl-closure:agent-diva-sandbox/src/guardian.rs
  → for_ask / OnFailure 风险预判 / UnlessTrusted 分臂
git show feat/m3-hitl-closure:agent-diva-tools/src/shell.rs
  → with_guardian_and_exec_policy
git show feat/m3-hitl-closure:agent-diva-gui/.../ChatView.vue
  → PERMISSION_MODE_KEY localStorage
```

## 未执行

- 未跑全量 `just ci`（审查只读为主；无代码修改需编译门禁）。
- 未启动 GUI / 真实 LLM smoke。
- 未逐行读完 `agent_loop.rs` 全部 3k+ 行；deferred 行为以 registry + 专用测试 + 关键段为准。

## 可复现命令

```powershell
git rev-parse HEAD
git merge-base --is-ancestor 9fb02d8a HEAD; echo $LASTEXITCODE
git merge-base --is-ancestor 3ce9eba2 HEAD; echo $LASTEXITCODE
git blame -L 348,390 agent-diva-sandbox/src/guardian.rs
rg "mount_tool|tool_discovery_v1" -g "*.rs"
rg "command-approval-requested" agent-diva-gui
```
