# 提案 04：全仓库 通道 (Channels) 与 提供者 (Providers) 死代码专项清理

## 1. 残留代码现状与定位

经过对 `agent-diva-channels` 和 `agent-diva-providers` 的深度审计，发现以下问题：

### 1.1 逻辑重复导致原函数变成死代码 (`email.rs`)
- **文件路径**: [`agent-diva-channels/src/email.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-channels/src/email.rs#L115-L130)
- **残留原因**: 辅助函数 `parse_email` 和 `html_to_text` 在生产轮询 `check_inbox` (L234-L253) 中被直接手写内联重写，导致原辅助函数全仓仅在单测中调用，生产代码中变成死代码并被 `#[allow(dead_code)]` 掩盖。

### 1.2 未接入的异步任务与废弃函数 (`telegram.rs`)
- **文件路径**: [`agent-diva-channels/src/telegram.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-channels/src/telegram.rs#L45-L395)
- **残留原因**: 
  - `start_typing` 后台循环打字指示器任务从未在任何消息处理流程中触发，`typing_tasks` 始终为空；
  - 迁移 Teloxide 后遗留的 `handle_text_message` 废弃函数（L246）被压制；
  - `TelegramHandler.proxy` 字段从不读取。

### 1.3 内联格式化忽略通用方法 (`nextcloud_talk.rs`)
- **文件路径**: [`agent-diva-channels/src/nextcloud_talk.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-channels/src/nextcloud_talk.rs#L44-L200)
- **残留原因**: 定义了 `ocs_base(&self)`，但在所有 API 请求中均手动手写格式化拼接 `format!("{}/ocs/v2.php/...", base_url)`，致使 `ocs_base` 被标记 `#[allow(dead_code)]`。

### 1.4 无用 Payload 结构体字段与错误结构
- **涉及文件**: `dingtalk.rs`, `qq.rs`, `feishu.rs`, `discord.rs`, `ollama.rs`
- **残留原因**:
  - `qq.rs`: `SessionStartLimit` 与 `GatewayInfo.shards` 解析后从未使用；
  - `ollama.rs`: 原生 `/api/chat` 请求误带入了 OpenAI 专有的 `stream_options` 节点；
  - `whatsapp.rs`: 过时的 Python/Bridge 路径注释（L6-L7）与 Map 二次包装冗余转换。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [REFRACTOR & DELETE]
1. 重构 `email.rs` 的 `check_inbox`，统一调用 `parse_email`，消除重复逻辑与 `#[allow(dead_code)]`。
2. 激活或清理 `telegram.rs` 的 `start_typing` 任务，删除废弃的 `handle_text_message` 函数与无用 `proxy` 字段。
3. 统一 `nextcloud_talk.rs` 的 URL 拼接逻辑至 `ocs_base()`。
4. 清理 `qq.rs` / `feishu.rs` / `dingtalk.rs` 中未使用的 Payload 字段，修正 `ollama.rs` 中的结构混淆。

---

## 3. 收益与风险评估
- **预期收益**：消除冗余数据解析；修复/激活打字状态功能；清理大量压制标记。
- **风险分析**：极低。`just check` 可确保无编译告警。
