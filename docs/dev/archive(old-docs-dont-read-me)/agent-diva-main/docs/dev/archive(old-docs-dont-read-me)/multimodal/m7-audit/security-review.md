# M7 多模态安全审计报告

**审计日期**: 2026-06-01
**审计范围**: 文件附件上传 → 图片嵌入 → LLM 请求 → Session 持久化 全链路
**审计文件**:
- `agent-diva-providers/src/base.rs`
- `agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva-core/src/session/store.rs`
- `agent-diva-core/src/attachment.rs`
- 辅助: `agent-diva-files/src/{manager,backend,storage,index,config}.rs`

---

## 1. 路径遍历 (Path Traversal)

**严重级别**: 🟢 低风险

**结论**: **安全**。file_id → 文件路径的解析链路不存在路径遍历漏洞。

**证据**:
- `FileManager::get(file_id)` 通过 `SqliteIndex::get(id, false)` 进行 SQLite 参数化查询（`bind(id)`），file_id 仅作为数据库主键查找，不直接拼接文件路径。
- 存储路径由系统计算：`hash_to_path()` 将 SHA256 哈希拆分为 `ab/cdef123...` 的两级目录结构，路径来源于数据库中存储的 `path` 列（由 `compute_hash()` 生成），非用户输入。
- `LocalStorageBackend::read(relative_path)` 使用 `self.data_dir.join(relative_path)`，其中 `relative_path` 来自数据库索引，非来自 `msg.media`。
- 即使恶意用户构造 `file_id = "../../etc/passwd"`，SQLite 查询仅返回 `None`（无匹配记录），不会产生文件读取。

---

## 2. 大小限制 (Size Limit)

**严重级别**: 🟢 低风险

**结论**: **5MB 检查在所有图片读取路径均生效**，且有双层校验。

**证据**:
- `loop_turn.rs:25` 定义 `MAX_VISION_IMAGE_SIZE = 5 * 1024 * 1024`。
- `resolve_image_file_to_data_uri()` 执行两次检查：
  1. **元数据层**（第 654-661 行）：`handle.metadata.size > MAX_VISION_IMAGE_SIZE` → 拒绝。
  2. **实际字节层**（第 669-675 行）：`bytes.len() as u64 > MAX_VISION_IMAGE_SIZE` → 拒绝。防止元数据被篡改绕过。
- `assemble_current_message_content()` 中内联文本附件检查 `size <= MAX_INLINE_ATTACHMENT_SIZE (100KB)`。
- **上传阶段**另有 `FileManager::store()` 中的 `config.max_file_size`（默认 500MB），与 vision 路径的 5MB 限制独立。这不构成安全问题——上传允许大文件，但发给 LLM 前强制 5MB 限制。

---

## 3. Data URI 注入 (Base64 Injection)

**严重级别**: 🟢 低风险

**结论**: **安全**。data URI 构造无注入向量。

**证据**:
- `resolve_image_file_to_data_uri()` 第 677-681 行：`format!("data:{};base64,{}", mime_type, BASE64_STANDARD.encode(bytes))`。
  - `mime_type` 经 `is_supported_vision_mime()` 白名单校验（仅 `image/png | image/jpeg | image/webp`），不可能包含注入字符。
  - Base64 字符集仅含 `[A-Za-z0-9+/=]`，不包含 JSON 特殊字符（`"`, `\`, `{` 等），不存在 JSON 注入风险。
- `MessageContentPart::ImageData` 变体直接透传 `data_uri` 字符串，但该值来自前端构造的 `ImageData`，经 `serde` 反序列化，不存在额外注入路径。

---

## 4. Session JSONL：图片 bytes 隔离

**严重级别**: 🟢 低风险

**结论**: **图片原始字节不会写入 Session JSONL**。

**证据**:
- `ChatMessage.attachments` 字段类型为 `Option<Vec<FileAttachmentRef>>`，而 `FileAttachmentRef` 仅包含 `file_id`、`filename`、`mime_type`、`size` 四个元数据字段（`attachment.rs:46-59`），**不包含 bytes、preview、base64 等字段**。
- `save_turn()` 第 860-869 行：用户消息通过 `ChatMessage::with_attachments(role, content, attachments)` 保存，其中 `attachments` 是 `Vec<FileAttachmentRef>`（由 `resolve_attachment_refs()` 生成）。
- 测试 `test_resolve_attachment_refs_reads_metadata_without_bytes`（第 1132-1168 行）明确验证：序列化后的 JSON 不包含 `"not persisted in session"`、`"preview should not be copied"`、`"base64"`、`"bytes"`。
- 测试 `test_chat_message_attachment_round_trip`（`store.rs:223-243`）验证序列化不含 base64/bytes/preview。

---

## 5. MIME 白名单

**严重级别**: 🟢 低风险

**结论**: **非 PNG/JPEG/WebP 格式被正确拒绝**。

**证据**:
- `is_supported_vision_mime()`（第 684-686 行）：严格匹配 `"image/png" | "image/jpeg" | "image/webp"`。
- 该检查在 `resolve_image_file_to_data_uri()` 第 647 行执行，在文件读取和 base64 编码**之前**。不支持的 MIME 类型直接返回 `UnsupportedMime` 错误。
- 测试 `test_prepare_messages_for_openai_vision_rejects_unsupported_mime`（第 1419-1456 行）验证 `image/svg+xml` 被拒绝。
- `assemble_current_message_content()` 中，`image/*` 类型仅创建 `ImageFile` 引用（不读取字节），MIME 白名单在后续 `prepare_messages_for_openai_vision` 阶段才强制执行。非图片 MIME 类型走内联文本或占位符路径。

---

## 6. 错误信息泄露

**严重级别**: 🟡 中风险

**结论**: **部分错误消息可能泄露内部 file_id 给 LLM**。

**证据**:
- `VisionMessagePreparationError::user_message()`（第 531-544 行）返回的是安全的静态用户消息，**不包含** file_id 或内部路径。✅ 安全。
- `assemble_current_message_content()` 第 747-749 行：`format!("[File: {} (error reading: {})]", handle.metadata.name, e)` — 错误信息包含文件名和内部错误详情，会作为用户消息的一部分发送给 LLM。⚠️ **低风险泄露**。
- 第 765-768 行：`format!("[Attachment: {} (not found - {})]", file_id, e)` — 错误信息直接暴露 file_id 给 LLM。⚠️ **低风险泄露**。
- 第 300-307 行：工具调用参数预览（截断 200 字符）通过 `info!()` 写入日志，不发送给外部。✅ 安全。
- 第 351-355 行：工具参数序列化失败的 `format!("Error: ...")` 包含 serde 错误详情，发送给 LLM。⚠️ **低风险泄露**。

**建议**: 对 LLM 可见的错误消息进行脱敏，移除内部 error details 和 file_id，改为通用占位符。

---

## 7. Panic 风险 (unwrap/expect)

**严重级别**: 🟢 低风险

**结论**: **生产代码无未保护的 unwrap/expect**。

**证据**:
- `loop_turn.rs` 生产代码中的 `unwrap` 仅出现在第 105 行：`response.prompt_block.unwrap()`，此前已通过 `if response.prompt_block.is_some()` 守卫（第 104 行），安全。
- 第 254 行使用 `unwrap_or_else()`（非 panic 的默认值模式）。
- 第 301 行 `serde_json::to_string(&tool_call.arguments).unwrap_or_default()` — 安全的降级处理。
- 所有其他 `unwrap()`/`expect()` 调用均位于 `#[cfg(test)]` 块中。
- `VisionMessagePreparationError` 实现了 `std::error::Error`，错误通过 `?` 传播，无 panic。

---

## 8. 并发安全

**严重级别**: 🟢 低风险

**结论**: **当前架构下并发安全**。

**证据**:
- `FileManager` 内部使用 `Arc<SqliteIndex>`，`SqliteIndex` 基于 `SqlitePool`（`max_connections(5)`），`SqlitePool` 是 `Send + Sync`，通过连接池天然支持并发读写。
- `SqliteIndex::update_ref_count()` 使用读-然后-写模式（先 SELECT 再 UPDATE），在高并发场景下理论上存在 TOCTOU 竞态，但 SQLite 的 WAL 模式下单写者特性保证了数据一致性。
- `AgentLoop::process_inbound_message_inner(&mut self, ...)` 需要 `&mut self`，同一 AgentLoop 实例不会并发处理消息。
- `Session` 结构体未使用 `Arc<Mutex<>>` 包装，但在当前架构中由 `AgentLoop` 独占访问（`&mut self`），无并发读写风险。

---

## 整体评分

| # | 审查项 | 严重级别 | 状态 |
|---|--------|----------|------|
| 1 | 路径遍历 | 🟢 低 | 安全 |
| 2 | 大小限制 | 🟢 低 | 安全 |
| 3 | Data URI 注入 | 🟢 低 | 安全 |
| 4 | Session JSONL 字节隔离 | 🟢 低 | 安全 |
| 5 | MIME 白名单 | 🟢 低 | 安全 |
| 6 | 错误信息泄露 | 🟡 中 | 需改进 |
| 7 | Panic 风险 | 🟢 低 | 安全 |
| 8 | 并发安全 | 🟢 低 | 安全 |

### 综合安全评分: **8 / 10**

**扣分原因**:
- **-1 分**: 错误消息在 `assemble_current_message_content()` 中泄露内部 file_id 和 error details 给 LLM（第 747-749、765-768 行）。虽然 LLM 本身不是攻击者，但这些信息可能出现在 LLM 回复中被用户看到，或在 prompt injection 场景下被利用。
- **-1 分**: 文本附件内联路径（第 729 行）使用 `handle.metadata.size` 做大小检查但未对实际读取字节做二次校验（与 vision 路径的双层校验不一致）。元数据被篡改时可能读取超限内容。

### 改进建议（优先级排序）

1. **P1 — 错误消息脱敏**: 将 `assemble_current_message_content()` 中的错误格式化替换为通用占位符，不暴露 `file_id` 和内部 `error` 详情。
2. **P2 — 文本附件大小二次校验**: 在 `file_manager.read()` 返回后检查 `bytes.len() <= MAX_INLINE_ATTACHMENT_SIZE`，与 vision 路径保持一致。
3. **P3 — 审计日志**: 对 file_id 查找失败和大小超限事件增加结构化审计日志（`tracing::warn!` with structured fields），便于异常检测。
