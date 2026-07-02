# M7 白盒审计计划

> 日期：2026-06-01
> 阶段：M7 验证验收 — 代码审计
> 范围：M1~M6 多模态图片识别全链路

## 基线信息

| 项目 | 值 |
|------|-----|
| Branch | main (ahead 4 commits) |
| HEAD | `fdb7bb8 feat: add vision request preparation` |
| 改动 commits | `3781f2a` (M1), `30e683f` (M2), `547bd68` (M3), `fdb7bb8` (M4+M5) |
| 改动 crates | agent-diva-providers, agent-diva-agent, agent-diva-core, agent-diva-gui |
| 净增行数 | ~2000 insertions, ~170 deletions, 38 files |

## 审计维度

### A — 架构与数据流 (Architecture)

审查 crate: `agent-diva-providers`, `agent-diva-agent`

关键文件:
- `agent-diva-providers/src/base.rs` — MessageContent / MessageContentPart 类型契约
- `agent-diva-providers/src/litellm.rs` — OpenAI-compatible vision 序列化
- `agent-diva-agent/src/agent_loop/loop_turn.rs` — 图片附件组装
- `agent-diva-agent/src/context.rs` — ContextBuilder 变更

审查问题:
1. MessageContent enum 设计是否合理？Text / Parts 分支是否有遗漏路径？
2. MessageContentPart 的 ImageFile / ImageUrl / ImageData 三种变体是否完备？
3. Agent loop 中图片附件组装为同一条 user message 的逻辑是否正确？
4. Vision 请求准备（resolve file_id → data URI）的路径是否完整？
5. 错误处理：file_id 不存在、MIME 不支持、文件过大时行为是否正确？
6. 模块间耦合是否合理？有没有循环依赖？

### B — 安全与鲁棒性 (Security)

审查 crate: 全部改动文件

审查问题:
1. 文件路径遍历攻击：resolve file_id → 读取文件的路径是否安全？
2. 图片大小限制：5MB 限制是否在所有路径生效？能否被绕过？
3. Data URI 注入：base64 编码内容是否安全嵌入 JSON？
4. Session JSONL 安全性：确认图片 bytes 不会写入 JSONL 持久化
5. Model ID 安全性：native endpoint 不走 LiteLLM 前缀重写
6. MIME 白名单：PNG/JPEG/WebP 外的格式是否正确拒绝？
7. 错误信息泄露：错误消息是否暴露内部路径或 file_id？
8. Panic 风险：unwrap/expect 是否有合理错误处理替代？

### C — 集成与兼容性 (Integration)

审查 crate: `agent-diva-core` (session), `agent-diva-gui`

审查问题:
1. ChatMessage.attachments 新增字段与旧 session JSONL 兼容性
2. FileAttachmentRef 序列化/反序列化 round-trip
3. text-only provider 路径是否完全不受影响？
4. Message::user("text") 老构造函数是否仍可用？
5. GUI 附件 chip 显示逻辑是否正确区分图片/普通文件？
6. GUI vision 模型白名单与后端是否一致？
7. 前端警告与后端 enforcement 之间是否有 gap？
8. 跨 crate 接口契约是否一致（InboundMessage.media → agent loop → provider）？

## 输出文件

| 子代理 | 输出路径 |
|--------|---------|
| A — 架构 | `docs/dev/multimodal/m7-audit/architecture-review.md` |
| B — 安全 | `docs/dev/multimodal/m7-audit/security-review.md` |
| C — 集成 | `docs/dev/multimodal/m7-audit/integration-review.md` |
| 汇总 | `docs/dev/multimodal/m7-audit/summary.md` |
