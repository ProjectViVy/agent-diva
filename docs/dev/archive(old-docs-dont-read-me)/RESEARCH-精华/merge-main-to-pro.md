# Main → Pro 合并精华（压缩版）

> 原始：`merge-main-to-pro-plan.md`（202 行）
> 核心：stash → force-merge → reapply，main 后端 > pro 后端

---

## 策略

```
1. 导出 pro 独有后端代码为补丁
2. git checkout main -- <全部除 agent-diva-gui/>
3. 打回补丁，解决冲突
4. 单独处理 GUI 冲突
```

## Pro 独有保留项

| 文件 | 内容 | 合并后操作 |
|------|------|----------|
| `agent-diva-core/src/usage/*.rs` | TokenUsageRecord、Budget、Writer、Query | 直接放回 |
| `agent-diva-manager/src/token_stats.rs` | 6 个 HTTP API handler | 直接放回 |

## 关键冲突解决

| 文件 | 解决方式 |
|------|----------|
| `core/src/lib.rs` | 添加 `pub mod usage` |
| `core/src/error.rs` | 添加 `Database` 变体 |
| `manager/src/lib.rs` | 保留 `token_stats` + `file_service` |
| `manager/src/server.rs` | 追加 token_stats 路由 |
| `manager/src/state.rs` | 合并 token 变体 + UploadFile 变体 |

## GUI 处理

- **Tauri Rust**：以 main 为基础，追加 pro 的 token stats 命令
- **Vue 前端**：以 pro 为基础，集成 main 的文件上传 UI

## 验证

```bash
cargo build --all
cargo clippy --all -- -D warnings
cargo test --all
cd agent-diva-gui && npm run build
```

## 原始文档

- `merge-main-to-pro-plan.md`
