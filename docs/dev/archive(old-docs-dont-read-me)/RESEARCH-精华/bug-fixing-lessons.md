# Bug 修复经验精华（压缩版）

> 原始：`bug-fixing-lessons-learned.md`（268 行）
> 核心：Windows proxy 拦截 + 路径不一致 = 两大隐形杀手

---

## Bug 1: GUI "Offline" / Bad Gateway

- **根因**：Windows HTTP proxy 拦截 localhost（Clash/V2Ray/企业环境）
- **修复**：`reqwest::Client::builder().no_proxy()`
- **文件**：`agent-diva-gui/src-tauri/src/app_state.rs`
- **教训**：本地服务永远用 `.no_proxy()`

## Bug 2: 文件上传成功但 AI 无法读取

- **根因**：upload 用 `%LOCALAPPDATA%`，read 用 `~/.agent-diva`
- **修复**：统一 `dirs::data_local_dir()`
- **文件**：`agent-diva-agent/src/agent_loop/loop_turn.rs`
- **教训**：路径计算必须集中化，不同组件不能各自算

## 预防清单

1. [ ] 创建共享 `paths.rs` 模块
2. [ ] 文件上传→读取端到端集成测试
3. [ ] 文档注明 `.no_proxy()` 要求
4. [ ] 运行时日志记录解析后的路径
5. [ ] 文件存在性检查 + 友好错误信息

## 原始文档

- `agent-diva-main/docs/dev/archive/bug-fixing-lessons-learned.md`
