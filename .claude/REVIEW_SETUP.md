# Claude Code 本地审查流程

## 快速开始

### 方式1：手动审查单个文件
```bash
# 使用 Python 脚本审查
python .claude/scripts/claude-review.py src/main.rs

# 或使用 Shell 脚本
./.claude/scripts/review.sh src/main.rs
```

### 方式2：作为 Claude 指令
在 Claude Code 中直接说：
```
帮我审查这个文件的改动
```

我会自动运行审查检查清单。

## 审查检查清单

见 `.claude/scripts/pre-review-checklist.md`

关键检查项：
- [ ] **内存安全**：大文件处理使用流式读取
- [ ] **数据库**：迁移脚本幂等性、软删除标记清除
- [ ] **安全**：文件上传大小限制、路径验证
- [ ] **架构**：避免多个静态实例、代码复用

## 当前发现的问题模式

### 1. 静态 FileManager 实例重复
**位置**：`file_service.rs`, `attachment.rs`
**问题**：多个 OnceCell<FileManager> 导致数据不一致
**修复**：统一使用 Arc<FileManager> 通过 AppState 传递

### 2. 存储路径计算重复
**位置**：`agent_loop.rs`, `file_service.rs`, `attachment.rs`
**代码**：`dirs::data_local_dir()?.join("agent-diva").join("files")`
**修复**：提取为共享辅助函数

### 3. unwrap_or_default 隐藏错误
**位置**：`index.rs:76`
**问题**：数据库错误被静默忽略
**修复**：显式处理或使用 ? 传播

## 与 GitHub Gemini Code Assist 配合

1. 本地开发时使用本流程预审查
2. 推送后等待 Gemini Code Assist 自动审查
3. 如需手动触发：在 PR 评论输入 `/gemini review`

## 配置

编辑 `.claude/review-config.toml` 自定义检查规则。
