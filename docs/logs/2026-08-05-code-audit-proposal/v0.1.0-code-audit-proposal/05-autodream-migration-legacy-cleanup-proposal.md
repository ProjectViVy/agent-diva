# 提案 05：收口 AutoDream 历史格式兼容逻辑与 Migration 边界

## 1. 残留代码现状分析

### 1.1 涉及文件与位置
- [`agent-diva-autodream/src/service.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-autodream/src/service.rs#L310-L325)
- [`agent-diva-migration/src/memory_migration.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-migration/src/memory_migration.rs#L20-L40)

### 1.2 背景与问题点
1. 在 AutoDream 完成 E0..E7 闭环后，AutoDream 运行记录具有确定性的 Schema 和恢复状态机。但 `service.rs` 中仍保留对旧版本不完整 AutoDream 记录的特殊 `LegacyIncomplete` 代码分支。
2. `agent-diva-migration` 模块包含早期从旧架构迁移 session/memory 的遗留代码，其中包含了被 `#[allow(dead_code)]` 压制的冗余转换辅助函数。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [MODIFY & DELETE]

1. **`agent-diva-autodream`**：
   - 规范化历史 AutoDream 运行记录的反序列化过程，遇到损坏或旧格式时统一触发 Fail-closed 安全拒绝逻辑，移除多余的兼容转换包装。

2. **`agent-diva-migration`**：
   - 彻底清理 Migration 内部未使用的结构体和兼容转换函数，保持 Migration CLI 仅聚焦于标准 Workspace Identity 校验与 Laputa 数据迁移。

---

## 3. 收益与风险评估

- **预期收益**：保证 AutoDream 引擎状态机的纯净；消除 Migration Crate 的测试和构建负担。
- **风险分析**：低。现有自动化集成测试已覆盖完整生命周期。
- **验证方法**：运行 `cargo test -p agent-diva-autodream` 与 `cargo test -p agent-diva-migration`。
