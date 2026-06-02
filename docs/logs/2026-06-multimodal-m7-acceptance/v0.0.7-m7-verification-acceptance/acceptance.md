# M7 验收报告 — 多模态图片识别

> **里程碑**: MM-ACCEPT（M7）— 验证、验收与发布准备
> **版本**: v0.0.7
> **日期**: 2026-06-01
> **验收人**: 交付负责人
> **状态**: ✅ 有条件通过（pending GUI smoke）

---

## 1. 产品验收清单

| # | 验收项 | 状态 | 依据 |
|---|--------|------|------|
| 1 | 用户能在 GUI 附加 PNG/JPEG 图片并输入问题 | ✅ 通过 | M6-A1~A2 实现；`ChatView.vue` 支持附件 chip、file_id 传输；集成审计 #5/#10 通过 |
| 2 | 后端收到图片 file_id | ✅ 通过 | `InboundMessage.media` 携带附件 ID；`file_service.rs` 存储上传文件；集成审计 #4 通过 |
| 3 | vision-capable 模型请求含 text+image 内容块 | ✅ 通过 | M5-A1~A2 实现；`MessageContent::Parts` 包含 `Text` + `ImageFile`/`ImageData`；providers vision 测试全部通过 |
| 4 | assistant 回答反映图片内容 | ⏳ 待验证 | 需 GUI smoke 端到端验证（见第 3 节） |
| 5 | text-only 模型不会收到 image payload | ✅ 通过 | M4-A1~A3 实现；`ModelCapabilities::vision` guard 在 agent loop 中过滤图片；providers 测试 `test_sanitize_messages_cleans_text_parts_and_preserves_images` 通过 |
| 6 | reload session 后附件元数据保留 | ✅ 通过 | M2-A2~A3 实现；`ChatMessage.attachments: Option<Vec<FileAttachmentRef>>`；集成审计 #9 reload session 元数据保留测试通过 |

**产品验收结论**: 6 项中 5 项通过，1 项待 GUI smoke 验证。

---

## 2. 工程验收清单

| # | 验收项 | 状态 | 依据 |
|---|--------|------|------|
| 1 | 旧文本消息和 session JSONL 兼容 | ✅ 通过 | `#[serde(default)]` 自动填充 `None`；集成审计 #1 旧 session JSONL 反序列化通过 |
| 2 | `Message::user("text")` 旧构造不破坏 | ✅ 通过 | M1-A1 实现；`MessageContent::Text` 为默认变体；集成审计 #3 text-only provider 路径通过 |
| 3 | native provider model ID 不被误加前缀 | ✅ 通过 | M5-A4 实现；providers 测试 `test_build_stream_request_keeps_openai_compatible_image_url_parts` 验证 model 字段不变 |
| 4 | 图片 bytes 不写入 session JSONL | ✅ 通过 | M2-A1 定义 `FileAttachmentRef` 仅含元数据（file_id/filename/mime_type/size）；安全审计 #4 通过 |
| 5 | 大图片/未知 MIME/file_id 不存在有可解释错误 | ✅ 通过 | M3-A3/M5-A3 实现；5MB 双层校验（元数据+实际字节）；MIME 严格白名单；agent attachment 测试覆盖大小限制、MIME 拒绝、file_id 不存在场景 |
| 6 | 文档与 docs/logs 完整 | ✅ 通过 | M7-A7 日志归档完成；审计报告 5 份（计划/架构/安全/集成/汇总）齐全；本文档为 acceptance 收尾 |

**工程验收结论**: 6 项全部通过。

---

## 3. 待完成项（GUI Smoke）

M7-A6 GUI smoke 测试尚未执行，需用户手动完成以下验证流程：

```text
1. 启动 agent-diva GUI。
2. 选择 vision-capable 的 OpenAI-compatible provider/model（如 gpt-4o）。
3. 上传一张 PNG 或 JPEG 图片。
4. 输入问题："这张图里是什么？"并发送。
5. 确认 provider request 中包含 structured image block（text + image_url）。
6. 确认 assistant 回答基于图片内容（而非通用拒绝或占位文本）。
7. 重载会话（reload session），确认用户历史消息仍显示图片附件标记。
```

**阻塞等级**: 非阻塞。工程验收全部通过，代码链路已贯通，GUI smoke 为产品体验最终确认。

---

## 4. 审计评分汇总

| 维度 | 评分 | 状态 |
|------|------|------|
| 自动化测试 | — | ✅ 多模态相关测试 100% 通过（唯一失败项为预存 skills 加载测试，与多模态无关） |
| 架构审计 | 8.5/10 | ✅ 无阻塞项 |
| 安全审计 | 8/10 | ✅ 无严重/高危，1 个中风险（错误消息脱敏）可后续修复 |
| 集成兼容性审计 | 8.5/10 | ✅ 无阻塞项，2 个 UX 改进建议 |

---

## 5. 合并前建议修复项（不阻塞验收）

| 优先级 | 问题 | 文件 | 行号 |
|--------|------|------|------|
| P1 | 错误消息脱敏（file_id 泄露给 LLM） | `loop_turn.rs` | 747-749, 765-768 |
| P1 | vision 模型白名单扩展（缺 Claude/Gemini） | `base.rs` | 59-63 |
| P1 | 前端阻止 text-only 模型发送图片 | `ChatView.vue` | 236-247 |
| P2 | 文本附件字节级二次校验 | `loop_turn.rs` | 729 |
| P2 | GIF MIME 支持 | `loop_turn.rs` | 684-686 |

---

## 6. 验收结论

### 判定：✅ 有条件通过

**通过条件**: GUI smoke 测试（M7-A6）完成并确认 assistant 回答能反映图片内容。

**依据**:

- 工程验收 6/6 项全部通过。
- 产品验收 5/6 项通过，仅"assistant 回答反映图片内容"待 GUI 端到端验证。
- M1~M6 全部里程碑退出条件满足。
- 自动化测试多模态相关项 100% 通过。
- 三维审计（架构/安全/集成）均无阻塞项。
- 代码链路从 GUI upload → file service → agent loop → provider serialization → session persistence 已贯通。

**后续行动**:

1. 执行 GUI smoke（M7-A6），确认后更新本报告第 1 节第 4 项状态。
2. 合并前优先修复 P1 建议项（错误消息脱敏、白名单扩展、前端阻止）。
3. 合并后进入 Phase A-PRE roadmap（扩展 vision 模型支持、GIF 支持、前端能力感知强化）。

---

> 本文档由 agent-diva 项目交付负责人签发，作为 M7 里程碑正式验收记录。
