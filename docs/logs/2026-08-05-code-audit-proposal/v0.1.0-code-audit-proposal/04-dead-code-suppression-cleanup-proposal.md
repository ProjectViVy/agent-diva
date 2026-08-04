# 提案 04：全仓库无用死代码（dead_code）压制标记专项清理

## 1. 残留代码现状分析

### 1.1 涉及文件与清单
经全局搜索，项目中有 40 余处显式使用 `#[allow(dead_code)]` 或 `#![allow(dead_code)]` 掩盖死代码的地方，典型位置包括：

1. **`agent-diva-channels`** (通讯通道模块)：
   - `dingtalk.rs` (L80, L92, L125)：未使用的钉钉消息 payload 结构体字段。
   - `email.rs` (L115, L130)：未使用的邮件 Server 辅助方法。
   - `manager.rs` (L44, L47)：未使用的 ChannelManager 调试函数。
   - `neuro_link.rs` (L32, L44, L51)：未调用的链路转换方法。
   - `qq.rs` (L137, L145)：未使用的 WebSocket 报文结构体字段。
   - `telegram.rs` (L45, L246, L377)：未使用的 Telegram Bot 命令及响应类型。
   - `whatsapp.rs` (L59)：未使用的回调封装。

2. **`agent-diva-gui/src-tauri`** (桌面端模块)：
   - `commands.rs` (L3776, L4470)：旧版的桌面命令辅助方法。
   - `embedded_server.rs` (L14, L41, L71)：未使用的 Server handle 方法。
   - `process_utils.rs` (L7 `#![allow(dead_code)]`)：整页压制警告的进程工具。

3. **`agent-diva-core` & `agent-diva-sandbox` & `agent-diva-autodream`**：
   - `agent-diva-core/src/planning/store.rs` (L1130, L1184, L1230, L1268)：测试辅助以外未被主流程调用的存取函数。
   - `agent-diva-sandbox/src/platform/windows.rs` (L33, L36, L39, L186)：未使用的 Win32 API 结构体定义。
   - `agent-diva-autodream/src/outputs.rs` (L306, L323)：未使用的序列化输出模型。

### 1.2 问题点
滥用 `#[allow(dead_code)]` 会屏蔽 Rust 编译器的真实死代码检查，导致遗留无用代码随着版本更迭越积越多，破坏了代码库的可读性与编译速度。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [DELETE / REFRACTOR]

1. **评估真实使用情况**：
   - 若字段/函数属于 FFI、外部 API 序列化必需字段（如 Win32 结构体或 QQ/Telegram 协议保留字段），移除 `#[allow(dead_code)]` 并加上下划线 `_` 前缀或属性注释（如 `#[serde(rename = "...")]`）。
   - 若为纯粹遗留的未调用函数/结构体，直接彻底删除。

2. **移除全页压制**：
   - 彻底删除 `process_utils.rs` 顶部的 `#![allow(dead_code)]` 全局压制声明。

---

## 3. 收益与风险评估

- **预期收益**：删除数百行无效代码；恢复 Clippy 对 Workspace 代码质量的硬约束能力。
- **风险分析**：极低。Clippy 与 Cargo test 会在编译期确保所有必需代码完好。
- **验证方法**：运行 `just check` (`cargo clippy --all -- -D warnings`)。
