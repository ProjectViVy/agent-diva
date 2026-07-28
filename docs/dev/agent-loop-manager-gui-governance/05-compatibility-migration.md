# 兼容性与迁移

## 1. 兼容承诺

整个治理周期默认保持：

- Rust workspace crate 名称和公开入口；
- CLI 命令与默认 gateway 行为；
- Manager 当前 `/api/*` path、method 和 JSON 字段；
- Tauri command 名称与前端可感知事件；
- session、Plan、TODO、config、Laputa、AutoDream 等持久化格式；
- debug 外部 gateway 与 release embedded gateway 两种桌面模式。

任何例外都必须独立 ADR、独立版本和独立迁移故事，不能作为“拆文件”的附带修改。

## 2. 内部迁移方式

### Agent Loop

保持 [AgentLoop](../../../agent-diva-agent/src/agent_loop.rs:123) 与现有构造/运行入口。旧 `loop_turn.rs` 在迁移期间调用新内部阶段；一个阶段完成并通过测试后，旧实现立即删除，不保留 feature flag 双轨。

### Manager

[handlers.rs](../../../agent-diva-manager/src/handlers.rs:1) 先改为 re-export/module index，使 [server.rs](../../../agent-diva-manager/src/server.rs:1) 不需要一次性改所有 import。DTO 拆分同样通过 `state.rs` 的短期 re-export 保持 source compatibility。

### GUI

新 `src/api/index.ts` 是目标入口；旧 [desktop.ts](../../../agent-diva-gui/src/api/desktop.ts:131) 暂时 re-export 新 domain adapter。每迁完一个 domain，就删除该 domain 的旧实现，避免新旧函数都能发请求。

Tauri command 文件拆分只改变 Rust module placement，不改变 `#[tauri::command]` 名称和 payload。Manager proxy 与 LOCAL command 必须分目录，但迁移期仍共用现有 invoke surface。

## 3. 禁止的迁移手段

- 不使用长期 `legacy` 目录保存仍编译的旧实现；
- 不同时向两个 store 写相同事实；
- 不让 GUI 根据新旧 response 猜测 schema；
- 不新增 `/v1` alias 作为过渡；
- 不把 deep 的 format-v7 importer 接入当前 runtime；
- 不以空数组、默认对象或 `Ok(())` 掩盖未绑定能力。

## 4. 回滚

每个切片只做内部结构变化，数据格式不变。回滚方式为撤销当期提交并重新构建，不需要数据迁移。若某阶段必须改变 persisted fact，则该阶段自动退出本计划，另立迁移方案。

## 5. 删除时机

只有同时满足以下条件才删除旧文件/导出：

1. 所有 call site 已迁移；
2. characterization、contract、GUI smoke 通过；
3. `rg` 证明无残余引用；
4. 迭代日志记录前后入口；
5. 单次 revert 能恢复。
