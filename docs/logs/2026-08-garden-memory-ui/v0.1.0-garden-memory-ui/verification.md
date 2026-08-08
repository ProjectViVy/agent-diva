# Garden 记忆工作台批次 · Verification

## 命令与结果（2026-08-09）

| 验证项 | 命令 | 结果 |
|---|---|---|
| Rust 编译（laputa） | `cargo check -p agent-diva-laputa` | ✅ 通过 |
| Rust 编译（manager） | `cargo check -p agent-diva-manager` | ✅ 通过 |
| Rust 编译（GUI tauri） | `cargo check -p agent-diva-gui` | ✅ 通过（3 个既有 warning） |
| BML 写边界守卫 | `cargo test -p agent-diva-laputa --test bml_boundary_guard` | ✅ 零违规 |
| 新单测 | `cargo test -p agent-diva-laputa --lib list_memories_returns_only_active_visible_records` | ✅ 1 passed |
| GUI 全量测试 | `npx vitest run`（agent-diva-gui） | ✅ 60 files / 459 tests passed |
| GUI 类型检查 | `npx vue-tsc --noEmit` | ✅ 通过 |

## 后端行为验证要点

- `list_memories` 单测覆盖：活跃记录可见、tombstone 记录排除、按 id 详情可读、FTS5 搜索命中正确
- 删除端点行为（代码路径）：`memory_remove` 生成 `Deprecation` PendingReview proposal 并经 `coordinator.submit`；记录在审批前**不删除**——与既有 G3 契约测试（`memory_remove_creates_governed_proposal`，typed_provider.rs）一致

## 新增测试

- `service.rs::wave4_tests::list_memories_returns_only_active_visible_records`
- `agent-diva-gui/src/components/memory/MemoryView.test.ts`（5 用例：列表渲染/详情/删除→proposal 跳转/取消不删除/错误态）

## 说明

- CLI wiremock 相关既有失败不在本批次改动范围（`CLI-WIREMOCK-502-PREEXISTING`）
- 全量 `just test`（cargo workspace）在提交前执行确认（见任务 #43 验证步骤）；GUI 冒烟（`gui-changes-need-gui-smoke`）需真机启动 GUI 验证，见 acceptance.md
