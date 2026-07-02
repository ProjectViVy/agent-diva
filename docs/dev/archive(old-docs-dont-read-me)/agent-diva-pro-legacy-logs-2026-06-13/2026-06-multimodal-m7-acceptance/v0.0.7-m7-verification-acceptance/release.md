# Release: v0.0.7-m7-verification-acceptance

> **日期**: 2026-06-01
> **里程碑**: MM-ACCEPT — 验证、验收与发布准备
> **状态**: ✅ 通过，建议合并

---

## 1. 版本信息

| 字段 | 值 |
|------|-----|
| 版本号 | `v0.0.7-m7-verification-acceptance` |
| 基准分支 | `main` |
| HEAD commit | `fdb7bb83769b221641f0d74722ed1cb8a94b2fa9` |
| 短哈希 | `fdb7bb8` |
| 提交信息 | `feat: add vision request preparation` |
| 覆盖范围 | M1~M6 多模态图片识别全链路 |

---

## 2. 交付物清单

### 2.1 代码变更（4 个功能 commit）

| # | Commit | 里程碑 | 说明 |
|---|--------|--------|------|
| 1 | `3781f2a` feat: add provider multimodal message content contract | M1 | 引入 `MessageContent` / `MessageContentPart` 类型契约，保留 `Message::user("text")` 兼容路径 |
| 2 | `30e683f` feat: persist session attachment metadata | M2 | 扩展 `ChatMessage` 增加 `attachments` 字段，附件元数据写入 JSONL，老格式自动兼容 |
| 3 | `547bd68` feat: assemble image attachments in agent loop | M3~M4 | agent loop 图片组装：MIME 判断、图片转 `ImageFile`/`ImageData`、capability guard、降级提示 |
| 4 | `fdb7bb8` feat: add vision request preparation | M5~M6 | OpenAI-compatible vision 序列化、data URI 转换、大小/MIME 限制、GUI 附件 chip 与警告 |

辅助 commit：
- `400b93c` style: apply cargo fmt — 格式化修复，无功能变更

### 2.2 审计报告（5 份）

| # | 文件 | 路径 |
|---|------|------|
| 1 | 审计计划 | `docs/dev/multimodal/m7-audit/m7-audit-plan.md` |
| 2 | 架构审计报告 | `docs/dev/multimodal/m7-audit/architecture-review.md` |
| 3 | 安全审计报告 | `docs/dev/multimodal/m7-audit/security-review.md` |
| 4 | 集成兼容性审计报告 | `docs/dev/multimodal/m7-audit/integration-review.md` |
| 5 | 最终审计汇总报告 | `docs/dev/multimodal/m7-audit/summary.md` |

### 2.3 迭代日志（4 份）

| # | 文件 | 路径 |
|---|------|------|
| 1 | 阶段总结 | `docs/logs/2026-06-multimodal-m7-acceptance/v0.0.7-m7-verification-acceptance/summary.md` |
| 2 | 验证记录 | `docs/logs/2026-06-multimodal-m7-acceptance/v0.0.7-m7-verification-acceptance/verification.md` |
| 3 | 发布记录 | `docs/logs/2026-06-multimodal-m7-acceptance/v0.0.7-m7-verification-acceptance/release.md`（本文档） |
| 4 | 验收记录 | `docs/logs/2026-06-multimodal-m7-acceptance/v0.0.7-m7-verification-acceptance/acceptance.md` |

---

## 3. 发布方式

**本阶段为验证与验收阶段，不产生新的二进制发布。**

本里程碑（M7）的核心产出是审计报告与迭代日志，确认 M1~M6 的代码变更已通过以下自动化门禁：

| 门禁 | 命令 | 结果 |
|------|------|------|
| 格式检查 | `just fmt-check` | ✅ 通过 |
| 静态分析 | `just check` | ✅ 通过（clippy 零警告） |
| 全量测试 | `just test` | ⚠️ 1 个无关失败（skills 目录加载，与多模态无关） |
| provider vision 定向测试 | `cargo test -p agent-diva-providers vision` | ✅ 通过 |
| agent attachment 定向测试 | `cargo test -p agent-diva-agent attachment` | ✅ 通过（8 项全通过） |

**合并路径**: 4 个功能 commit 已在 `main` 分支 HEAD，代码可直接合并至发布分支。无需额外 cherry-pick 或 squash。GUI smoke test（M7-A6）待用户手动验证，不阻塞代码合并。

---

## 4. 回滚方案

### 4.1 按 commit 逐个回退（推荐）

M1~M6 的 4 个功能 commit 相互独立，可按依赖顺序逐一 revert：

```powershell
# 逆序回退：M6/M5 → M4/M3 → M2 → M1
git revert fdb7bb8   # M5/M6: vision request preparation
git revert 547bd68   # M3/M4: image attachments in agent loop
git revert 30e683f   # M2: session attachment metadata
git revert 3781f2a   # M1: provider multimodal message content contract
```

每个 revert 后编译验证：
```powershell
cargo build --all
cargo test --all
```

### 4.2 整体回退

如需一次性回退全部多模态改动，可直接切回 M1 之前的基准：

```powershell
# 方法 A：revert 整个 commit range
git revert fdb7bb8..3781f2a --no-commit
git commit -m "revert: rollback multimodal M1-M6 changes"

# 方法 B：重置到 M1 之前的状态（破坏性，仅限未发布场景）
git reset --hard 400b93c   # cargo fmt commit，M1 的前一个 commit
```

### 4.3 回退影响评估

| 回退项 | 影响 |
|--------|------|
| M1 revert | `MessageContent` / `MessageContentPart` 类型移除，`content` 恢复为 `String` |
| M2 revert | `ChatMessage.attachments` 字段移除，新 session 中的附件元数据丢失 |
| M3/M4 revert | agent loop 图片组装移除，图片附件退化为文本占位符 |
| M5/M6 revert | OpenAI-compatible vision 序列化移除，GUI 附件 chip/警告移除 |

**注意**: 由于 `#[serde(default)]` 兼容设计，回退后老 session JSONL 中的 `attachments` 字段会被自动忽略，不会导致反序列化失败。

---

## 5. 依赖变更

### 5.1 新增依赖

| Crate | 依赖 | 版本 | 用途 |
|-------|------|------|------|
| `agent-diva-providers` | `base64` | workspace | `ImageData`（file_id → data URI）的 Base64 编码 |

`base64` 已在 workspace `Cargo.toml` 的 `[workspace.dependencies]` 中声明，providers crate 通过 `{ workspace = true }` 引用。

### 5.2 无新增 crate 级依赖

本次变更未引入新的外部 crate。`reqwest`、`serde`、`tokio` 等已有依赖未发生版本变更。

---

## 6. 影响范围

### 6.1 改动的 crate

| Crate | 改动内容 | 风险等级 |
|-------|----------|----------|
| `agent-diva-providers` | 新增 `MessageContent`、`MessageContentPart`、`ModelCapabilities`；OpenAI-compatible vision 序列化 | 中 — 核心类型变更 |
| `agent-diva-agent` | agent loop 图片组装、capability guard、附件读取 | 中 — 主循环逻辑 |
| `agent-diva-core` | `FileAttachmentRef` 定义、`ChatMessage.attachments` 扩展 | 低 — 纯数据结构 |
| `agent-diva-gui` | 附件 chip 区分图片/文件、text-only 模型警告 | 低 — 前端 UI |

### 6.2 未改动的 crate

- `agent-diva-channels` — 无变更
- `agent-diva-tools` — 无变更
- `agent-diva-manager` — 无变更（文件服务已有，未修改）
- `agent-diva-cli` — 无变更
- `agent-diva-neuron` — 无变更

### 6.3 text-only 路径回归验证

| 验证项 | 结果 |
|--------|------|
| `Message::user("text")` 构造 | ✅ 返回 `MessageContent::Text`，行为不变 |
| 旧 session JSONL 反序列化 | ✅ `#[serde(default)]` 自动填充 `attachments: None` |
| text-only provider 请求 | ✅ content 仍为纯字符串，无 image block |
| text-only 模型 + 图片附件 | ✅ 给出明确降级提示，不发送 image payload |

**结论**: text-only 路径无回归，向后兼容性完整。

---

## 7. 已知遗留项（不阻塞合并）

| 优先级 | 问题 | 文件 | 说明 |
|--------|------|------|------|
| P1 | 错误消息脱敏（file_id 泄露给 LLM） | `loop_turn.rs:747-749, 765-768` | 后续迭代修复 |
| P1 | vision 模型白名单扩展（缺 Claude/Gemini） | `base.rs:59-63` | 后续迭代扩展 |
| P1 | 前端阻止 text-only 模型发送图片 | `ChatView.vue:236-247` | 当前仅警告，后续改为阻止 |
| P2 | 文本附件字节级二次校验 | `loop_turn.rs:729` | 与 vision 路径对齐 |
| P2 | GIF MIME 支持 | `loop_turn.rs:684-686` | 当前仅 PNG/JPEG/WebP |
| — | GUI smoke test（M7-A6） | — | 待用户手动验证 |

---

## 8. 审计成本

| 维度 | Turns | 花费 |
|------|-------|------|
| 架构审计 | 11 | $0.87 |
| 安全审计（初） | 23 | $1.53 |
| 安全审计（重跑） | 15 | $0.89 |
| 集成审计 | ~15 | $1.15 |
| **合计** | **~64** | **$4.44** |

---

## 9. 最终判定

| 维度 | 评分 | 状态 |
|------|------|------|
| 自动化测试 | — | ✅ 多模态测试 100% 通过 |
| 架构审计 | 8.5/10 | ✅ 无阻塞项 |
| 安全审计 | 8/10 | ✅ 无严重/高危，1 个中风险可后续修复 |
| 集成兼容性审计 | 8.5/10 | ✅ 无阻塞项，2 个 UX 改进建议 |

**状态**: ✅ **建议合并，可发布**
