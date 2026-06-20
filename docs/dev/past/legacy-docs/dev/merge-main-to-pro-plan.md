# main → agent-diva-pro 合并方案 v2

> **版本**: v2.0  
> **日期**: 2026-06-02  
> **状态**: 方案设计 — 禁止修改代码  
> **策略**: stash → force-merge → reapply  

---

## 一、策略概述

```
┌─────────────────────────────────────────────────────────┐
│  1. 导出 pro 独有后端代码为补丁                           │
│  2. git checkout main -- <全部除 agent-diva-gui/>         │
│  3. 打回补丁，解决冲突                                    │
│  4. 单独处理 GUI 冲突                                    │
└─────────────────────────────────────────────────────────┘
```

**核心原则**：main 后端 > pro 后端（main 更成熟）。pro 的 GUI 和后端独有功能通过补丁保留。

---

## 二、pro 需要保留的占位符清单

### 2.1 全新文件（直接保存副本，合并后放回）

| 文件 | 内容 | 合并后操作 |
|------|------|----------|
| `agent-diva-core/src/usage/mod.rs` | usage 模块入口 | 直接放回，无冲突 |
| `agent-diva-core/src/usage/types.rs` | TokenUsageRecord、ModelPricing 等 | 直接放回 |
| `agent-diva-core/src/usage/budget.rs` | TokenBudget 预算控制 | 直接放回 |
| `agent-diva-core/src/usage/writer.rs` | 异步写入器 | 直接放回 |
| `agent-diva-core/src/usage/query.rs` | SQLite 查询服务 | 直接放回 |
| `agent-diva-manager/src/token_stats.rs` | 6 个 HTTP API handler | 直接放回 |

**不需要保存**：
- `agent-diva-core/src/security/policy.rs` — 空文件，main 有完整实现覆盖
- `agent-diva-manager/src/routes/ws_chat.rs` — 两分支都是空文件

### 2.2 修改文件（生成 unified diff，合并后打补丁）

| 文件 | pro 改了什么 | 冲突预估 |
|------|------------|---------|
| `agent-diva-core/src/lib.rs` | +`pub mod usage` | 🟡 与 main 的 attachment/security 导出合并 |
| `agent-diva-core/src/error.rs` | +`Database` 错误变体, +`From<rusqlite::Error>` | 🟢 main 未改此区域 |
| `agent-diva-core/Cargo.toml` | +rusqlite, +regex 依赖 | 🟡 main 也加了 deps，合并 |
| `agent-diva-manager/src/lib.rs` | +`pub mod token_stats` | 🟡 main 加了 file_service，合并 |
| `agent-diva-manager/src/manager.rs` | +runtime_control 模块和字段 | 🟡 main 加了 file_manager 字段 |
| `agent-diva-manager/src/manager/runtime_control.rs` | 展开的运行时控制逻辑 | 🟡 main 可能也有此文件 |
| `agent-diva-manager/src/server.rs` | +token_stats 路由 | 🟡 main 重构了 build_router + 加了 file upload 路由 |
| `agent-diva-manager/src/state.rs` | +token stats ManagerCommand 变体 | 🟡 main 加了 UploadFile 变体 |
| `Cargo.toml` (根) | 版本 0.4.1，独有 deps？ | 🟡 main 版本 0.4.9，+agent-diva-files |

---

## 三、执行步骤

### 步骤 1：保存 pro 占位符

```bash
# 在 agent-diva-pro 分支上执行

# 1a. 复制全新文件到临时目录
mkdir -p /tmp/pro-stash
cp -r agent-diva-core/src/usage /tmp/pro-stash/
cp agent-diva-manager/src/token_stats.rs /tmp/pro-stash/

# 1b. 生成修改文件的 unified diff
git diff main -- \
  agent-diva-core/src/lib.rs \
  agent-diva-core/src/error.rs \
  agent-diva-core/Cargo.toml \
  agent-diva-manager/src/lib.rs \
  agent-diva-manager/src/manager.rs \
  agent-diva-manager/src/manager/runtime_control.rs \
  agent-diva-manager/src/server.rs \
  agent-diva-manager/src/state.rs \
  Cargo.toml \
  > /tmp/pro-stash/backend-diff.patch

# 1c. 也保存 GUI 后端 diff（后续单独处理）
git diff main -- agent-diva-gui/src-tauri/ > /tmp/pro-stash/gui-tauri-diff.patch
```

### 步骤 2：强制合并 main（排除 GUI）

```bash
# 除了 agent-diva-gui/ 全部用 main 覆盖
git checkout main -- \
  Cargo.toml Cargo.lock CLAUDE.md .gitignore \
  agent-diva-core/ \
  agent-diva-agent/ \
  agent-diva-tools/ \
  agent-diva-providers/ \
  agent-diva-channels/ \
  agent-diva-cli/ \
  agent-diva-manager/ \
  agent-diva-neuron/ \
  agent-diva-service/ \
  agent-diva-migration/ \
  agent-diva-files/ \
  scripts/ \
  docs/ \
  .skills/

# 放回 pro 的全新文件
cp -r /tmp/pro-stash/usage agent-diva-core/src/
cp /tmp/pro-stash/token_stats.rs agent-diva-manager/src/
```

### 步骤 3：打回补丁（逐文件解决冲突）

```bash
# 尝试自动打补丁
git apply /tmp/pro-stash/backend-diff.patch

# 会有 reject，逐个手动解决以下文件：
```

**冲突解决清单**：

| 文件 | 解决方式 |
|------|---------|
| `agent-diva-core/src/lib.rs` | 在 main 版本基础上添加 `pub mod usage;` |
| `agent-diva-core/src/error.rs` | 在 main 版本基础上添加 `Database` 变体和 `From<rusqlite::Error>` |
| `agent-diva-core/Cargo.toml` | 在 main 版本基础上添加 rusqlite、regex 依赖 |
| `agent-diva-manager/src/lib.rs` | 同时保留 `pub mod token_stats;` 和 `pub mod file_service;` |
| `agent-diva-manager/src/manager.rs` | 结构体同时添加 `runtime_control_tx` 和 `file_manager` 字段；match 分支合并 |
| `agent-diva-manager/src/manager/runtime_control.rs` | main 也有此文件，用 pro 版本覆盖 |
| `agent-diva-manager/src/server.rs` | 在 main 的 `build_router` 中追加 pro 的 token_stats 路由（6 条） |
| `agent-diva-manager/src/state.rs` | `ManagerCommand` 枚举合并 pro 的 6 个 token 变体 + main 的 UploadFile 变体 |
| `Cargo.toml` | 用 main 的版本（0.4.9 + agent-diva-files workspace），确认 pro 无额外依赖 |

### 步骤 4：GUI 单独处理

GUI 层（`agent-diva-gui/`）不参与强制合并，单独处理：

#### 4a. Tauri Rust 后端

以 main 版本为基础：
- `lib.rs`、`commands.rs`、`app_state.rs`、`process_utils.rs`、`Cargo.toml`、`tauri.conf.json` → 使用 main
- 在 `lib.rs` 的 invoke_handler 中追加 pro 的 6 个 token stats 命令
- 在 `commands.rs` 中追加 pro 的 token stats 函数，**关键**：`state.api_base_url` → `state.api_base_url()` 全部替换

#### 4b. Vue 前端

以 pro 版本为基础（UI 架构更好）：
- `ChatView.vue` — 在 pro 基础上集成 main 的文件上传 UI
- `NormalMode.vue` — 使用 pro，透传 attachments，删除两个 backup 文件
- `ChannelsSettings.vue` — 使用 pro（CardView+Wizard），在 Wizard 中添 main 的飞书 `allow_from` 字段
- `GeneralSettings.vue` — 使用 pro，集成 main 的 close-to-tray
- `App.vue` — 使用 pro（splash 已启用），集成 attachments 参数
- `api/desktop.ts` — 合并
- 其他 — pro 为主

### 步骤 5：验证

```bash
# 编译
cargo build --all

# lint
cargo clippy --all -- -D warnings

# 测试
cargo test --all

# GUI 构建
cd agent-diva-gui && npm run build
```

---

## 四、关于启动动画

**无需操作**。两分支 splash 机制完全相同。pro 已启用（`App.vue` 中 `markSplashComplete()` 活跃），main 只是注释掉了调用。合并后保持 pro 的激活状态。

## 五、关于图像识别/多模态

**状态**：main 分支在 channel 层有图片下载+base64 编码（飞书 `fetch_image_marker`），但 provider 层没有结构化 `image_url` 内容块构造。图片数据以文本标记 `[IMAGE:data:...;base64,...]` 传入 LLM，是否被模型识别取决于具体 provider。
合并后此功能自然保留（来自 main 的 channels 代码），无需额外处理。

## 六、关于文件上传

合并后 main 的 `agent-diva-files` 完整系统自然引入。需确保：
1. GUI `ChatView.vue` 集成文件上传 UI
2. GUI `commands.rs` 的 `upload_file` 命令存在
3. Manager 的 `/api/files/upload` 路由存在

---

## 七、执行前置条件

1. **确认无未提交修改** — `git status` clean
2. **创建备份分支** — `git branch agent-diva-pro-backup`
3. **创建临时目录** — `mkdir -p /tmp/pro-stash`
4. **运行步骤 1 保存占位符**
5. **运行步骤 2-3 合并+打补丁**
6. **运行步骤 4 处理 GUI**
7. **运行步骤 5 验证**
