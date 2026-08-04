# 提案 05：清理 Migration 1389 行未挂载代码、Tools/Sandbox 及 AutoDream 残留

## 1. 残留代码现状与定位

经过对 `agent-diva-migration`, `agent-diva-sandbox`, `agent-diva-files` 及 `agent-diva-autodream` 的深度审计，发现以下重大残留点：

### 1.1 `agent-diva-migration` 源码树中积压的 1,389 行未挂载死文件
- **文件路径**: 
  - `agent-diva-migration/src/config_migration.rs` (805 行)
  - `agent-diva-migration/src/memory_migration.rs` (303 行)
  - `agent-diva-migration/src/session_migration.rs` (281 行)
- **残留原因**: 
  `agent-diva-migration/src/main.rs` 中仅声明了 `mod experience; mod typed_memory; mod workspace_identity;`。这 3 个文件**完全没有被 `main.rs` 包含挂载**，属于遗留在源码目录中、不被编译且已被 Laputa 架构废弃的旧版本 Python 迁移死代码（共计 1,389 行）。

### 1.2 `agent-diva-sandbox` 未调用的 API 与死常量
- **文件路径**: 
  - `agent-diva-sandbox/src/platform/linux.rs` (L124 `is_wsl()`)
  - `agent-diva-sandbox/src/platform/windows.rs` (L42 `WRITE_RESTRICTED`)
- **残留原因**: `is_wsl()` 仅在自身的单测中被调用，实际沙箱构建只使用 `is_wsl1()`；`WRITE_RESTRICTED` 常量带 `#[allow(dead_code)]`，因会导致 Windows Shell 初始化崩溃而被放弃使用。

### 1.3 `agent-diva-files` 未接入的 Metadata Extract Hook
- **文件路径**: [`agent-diva-files/src/hooks.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-files/src/hooks.rs#L814-L833) (`run_extract_metadata`)
- **残留原因**: 被标记 `#[allow(dead_code)]`。`FileManager` 存储流水线中仅调用了 `run_validate_metadata`，从未调用 `run_extract_metadata`。

### 1.4 `agent-diva-autodream` 旧 Run 恢复兼容逻辑
- **文件路径**: [`agent-diva-autodream/src/service.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-autodream/src/service.rs#L306-L323)
- **残留原因**: 对缺失 `orchestration` 编排元数据的早期历史 AutoDream Run 进行特殊拒绝打标。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [PHYSICAL DELETE & CLEAN]
1. **物理删除** `agent-diva-migration` 中未挂载的 3 个过时文件（精简 1,389 行）。
2. 删除 `linux.rs` 中未调用的 `is_wsl()` 及 `windows.rs` 中的死常量 `WRITE_RESTRICTED`。
3. 在 `FileManager::store` 中接入 `run_extract_metadata`，或清理未调用的 Hook 方法。
4. 收尾 AutoDream 历史旧格式 Run 的安全处理。

---

## 3. 收益与风险评估
- **预期收益**：极大瘦身 `agent-diva-migration` 目录（直接清理 1389 行）；消除沙箱与文件索引 Hook 的未用死代码。
- **风险分析**：无风险。未挂载文件本就不参与当前构建。
