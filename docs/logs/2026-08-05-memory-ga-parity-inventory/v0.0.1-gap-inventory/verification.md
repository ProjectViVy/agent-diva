# Verification — 残缺项盘点证据与方法

> 本迭代为 **静态代码/文档对照**，未运行端到端产品验收。  
> 目的：证明 inventory 中每个状态标记有可复核证据。

## 1. 对照方法

1. **GA 基线**：阅读 `.workspace/GenericAgent` 源码与 SOP（相对 monorepo 上级 `morediva/.workspace/GenericAgent`）。
2. **Diva 实现**：在 `agent-diva` workspace 内检索工具注册、MemoryProvider、Laputa、AutoDream。
3. **架构约束**：读取 `docs/architecture/laputa-memory-final-architecture.md`、`memory-framework-interfaces.md`。
4. **状态判定规则**：
   - ✅ 主路径代码存在且注释/测试表明可调用
   - 🟡 代码存在但未接通 agent 面 / 语义偏差 / 待验收
   - ❌ agent 或用户无可达路径
   - ⚠ 文档/prompt 与实现冲突
   - 🔒 架构明确禁止

## 2. GenericAgent 证据

| 结论 | 证据 |
|------|------|
| 记忆分层 L0–L4 | `memory/memory_management_sop.md` 层级描述 |
| 清理/ROI | `memory/memory_cleanup_sop.md` |
| 主动蒸馏工具 | `ga.py` → `do_start_long_term_update` |
| 工作记忆工具 | `ga.py` → `do_update_working_checkpoint` |
| WORKING MEMORY 注入 | `ga.py` → `_get_anchor_prompt` / `turn_end_callback` |
| L4 定时归档 | `reflect/scheduler.py` L4 cron（12h） |
| 历史重点挖掘 | `memory/L4_raw_sessions/salient_mining_sop.md` |
| 全局记忆注入 | `ga.py` → `get_global_memory` |

## 3. agent-diva 证据

### 3.1 Memory

| 结论 | 证据 |
|------|------|
| Provider 四生命周期 | `agent-diva-core/src/memory/provider.rs` trait `MemoryProvider` |
| Legacy 读写 MEMORY.md | `agent-diva-core/src/memory/manager.rs` |
| Legacy prefetch 失败 | 同文件 `prefetch` → `PrefetchStatus::Failed` |
| Typed 记录模型 | `agent-diva-core/src/memory/record.rs` |
| Consolidation 内部 save_memory | `agent-diva-agent/src/consolidation.rs`；仅内部 tools schema |
| 触发 consolidation | `agent-diva-agent/src/agent_loop/turn/finalize.rs` |
| **无** memory 工具模块 | `agent-diva-tools/src/` 目录无 memory 相关源文件 |
| 工具注册无 memory | `agent-diva-agent/src/tool_assembly.rs` 仅 file/shell/web/cron/plan… |
| Prompt 假承诺 memory tools | `agent-diva-agent/src/context.rs` 文案 “available memory tools” |
| mode 选择 / degraded | `agent-diva-agent/src/memory_boundary.rs` |
| 缺 config → Legacy | `agent-diva-core/src/config/schema.rs` `legacy_memory_config` |

### 3.2 Laputa

| 结论 | 证据 |
|------|------|
| proposal/apply service | `agent-diva-laputa/src/service.rs` |
| sync_turn → pending proposal | `agent-diva-laputa/src/memory_provider.rs` `create_turn_proposal` |
| file-first prefetch 空 | 同文件 `prefetch` 返回空/跳过类结果 |
| Typed open + startup render | `agent-diva-laputa/src/typed_provider.rs` |
| Typed sync_turn 委派 proposal | 同文件 `sync_turn` → `proposal_sink` |
| FTS/recall 骨架 | `agent-diva-laputa/src/recall.rs`, `typed_store.rs` |
| governed apply | `agent-diva-laputa/src/governed_apply.rs` |
| Manager/GUI 治理面 | `agent-diva-manager` laputa handlers；GUI `PersonaMemoryView` / laputa API |

### 3.3 AutoDream

| 结论 | 证据 |
|------|------|
| crate 生命周期 | `agent-diva-autodream/src/{service,worker,inputs,outputs,reflection,reports}.rs` |
| 候选进 Laputa | `outputs.rs` 创建 `EvolutionProposal` |
| Manager API | `agent-diva-manager/src/server.rs` `/api/autodream/runs*` |
| GUI 文案“已连通待最终验收” | `agent-diva-gui/src/locales/zh.ts` Evolution/AutoDream 描述 |
| 开放验收 | 根 `TODOLIST.md` GMH-52 等 |

### 3.4 架构

| 结论 | 证据 |
|------|------|
| GA 是设计参考非移植 | `docs/architecture/laputa-memory-final-architecture.md` |
| Agent 不可直接写 authority | `docs/architecture/memory-framework-interfaces.md` |
| AutoDream 永不 authority | 同上 + autodream 架构文档 |

## 4. 本迭代验证命令

```text
# 文档存在性
ls docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.1-gap-inventory/

# 静态确认无 memory tool（示例）
rg -n "memory" agent-diva-tools/src --glob "*.rs"   # 应无 Memory 工具定义
rg -n "available memory tools" agent-diva-agent/src/context.rs
rg -n "do_start_long_term_update|update_working_checkpoint" ../../.workspace/GenericAgent/ga.py
```

> 未执行 `just ci`：本迭代无 Rust 源码变更，CI 非必需。

## 5. 结果

| 检查 | 结果 |
|------|------|
| inventory 与源码对照可复核 | Pass |
| 关键缺口（无 agent memory tools）可静态证明 | Pass |
| 产品端到端 U1–U8 | **Not run**（属实施后验收） |
| 功能代码变更 | None |
