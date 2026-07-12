# 手动日报、周报、月报的 LLM 归纳改造计划

## 目标与问题

手动生成的日报、周报、月报目前以确定性模板拼接会话摘录、日报摘要和统计字段。这样能保留来源，但阅读结果会重复标题、会话 ID、欢迎语和原始片段；示例中的 2026-07-12 日报虽统计了 7 个会话与 96 条消息，却没有归纳用户真正完成的工作、主要诉求、风险或后续事项。

本计划将手动触发的三类报表改为“先收集可审计事实，后由 LLM 归纳呈现”的两层产物：正文是面向用户的中文归纳，证据与计数仍可追溯且默认折叠/置于文末。目标是让用户点击生成后直接得到可读、去重、可行动的报告，而非 raw 数据列表。

## 范围与边界

- 覆盖 Notebook 的手动 `daily`、`weekly`、`monthly` 触发；保留现有日报/周报由 AutoDream 持有、月报由 Report System 持有的存储所有权。
- 不改变 session 原文、证据 URI、报告原子写入、现有前端读取和固化（proposal）能力。
- LLM 仅基于本次收集的受限输入生成文字，不允许工具调用、联网、文件写入或把未证实内容写成事实。
- LLM 不可用、超时、输出无效或安全校验失败时，生成必须成功降级为现有确定性摘要，并明确标记，不得写入空报告或静默伪造归纳结果。
- 不在本次实施中改变自动定时月报的调度策略；其生成内核与手动路径共用后，定时路径可沿用同一能力。

## 当前链路与改造落点

| 报表 | 当前生成入口 | 当前输入/问题 | 目标改造 |
| --- | --- | --- | --- |
| 日报 | `agent-diva-autodream/src/rhythm.rs::generate_daily_for_date` | `SessionWindowDigest.items` 被直接逐条渲染 | 将经裁剪和去重的会话事实送入 LLM，生成当天工作主题、进展、决策和待跟进项 |
| 周报 | `generate_weekly_for_week` | 日报 `summary` 与缺口会话逐条拼接 | 将日报归纳与缺口会话统一为周级事实包，由 LLM 归纳趋势、成果、阻塞与下周重点 |
| 月报 | `agent-diva-autodream/src/monthly.rs`；GUI 另有历史复制实现 | 日报 summary/fallback 被模板化拼接，月度“synthesis”也只是截断连接 | 统一月报生成归属与实现后，使用日报归纳、缺口会话及统计生成月度复盘、主题演进、风险与建议 |
| GUI 手动触发 | `agent-diva-gui/src-tauri/src/commands.rs::trigger_notebook_report_generation` → manager `/autodream/runs` | 没有“归纳中/降级”可见状态 | 保持触发契约，返回并展示生成元数据与降级原因（如有） |

月报目前在 `agent-diva-autodream/src/monthly.rs` 和 `agent-diva-gui/src-tauri/src/notebook.rs` 各有一份近似的确定性生成逻辑。实施前先指定唯一生产调用方为 AutoDream/manager；GUI 只触发、读取和展示，避免两套 LLM 行为与 schema 演进分叉。

## 目标架构

```text
手动 Notebook 触发
        ↓
日期窗口 + 会话/下级报告收集（保留原始 EvidenceRef）
        ↓
ReportFactBundle（去重、限额、统计、缺口、可引用事实）
        ↓
ReportNarrativeGenerator（LLM；无工具、结构化输出、超时）
        ↓                         ↘ 失败/无效
CuratedReportNarrative              DeterministicFallbackNarrative
        ↓                         ↓
统一 Markdown renderer + frontmatter + Evidence References
        ↓
各自既有报告存储路径 → Notebook 展示/固化
```

### 领域模型与接口

在 `agent-diva-core` 增加与 provider 无关的报告契约，建议包括：

- `ReportPeriod`（Daily/Weekly/Monthly）、`ReportWindow`、`ReportFactBundle`、`ReportFact`、`ReportSourceCoverage`；事实保留 stable ID、来源、时间、简短内容和关联 evidence ID。
- `CuratedReportNarrative`：`executive_summary`、`themes`、`accomplishments`、`decisions`、`risks_or_blockers`、`next_actions`、`coverage_notes`，每个条目带 `evidence_ids`。
- `ReportGenerationMetadata`：`generation_mode`（`llm_curated`/`deterministic_fallback`）、模型标识（仅用于审计）、prompt/schema 版本、输入/输出 token（可得时）、耗时、失败原因（脱敏）。
- `ReportNarrativeGenerator` trait：输入为 `ReportFactBundle` 与明确的预算/语言/period，输出为结构化 narrative；不暴露工具注册表或任意 prompt 参数。

在 `agent-diva-providers` 实现一个复用现有聊天 provider 的适配器；配置选择当前已配置的报告/默认模型，模型请求遵守 native endpoint 使用原始 model ID 的规则。`agent-diva-autodream` 只负责收集、编排、验证和落盘，不直接依赖特定供应商。

## 输入控制、提示词与真实性

1. 收集阶段保留现有 session-window、日报读取和缺口恢复语义，但先过滤空欢迎语、重复首轮问候、纯工具目录输出和完全重复片段；不能删除有业务含义的用户消息。
2. 按 period 建立有限大小的事实包：优先完整保留最近/高信息量事实，超出预算时先做确定性、带 evidence ID 的分组压缩；记录截断数与覆盖度。
3. System prompt 要求：只使用提供事实；事实不足时明确写“信息不足”；禁止猜测完成状态、个人属性、时间和外部事件；输出中文 JSON，不输出 markdown/思维过程。
4. 每个 narrative 条目必须引用至少一个 `evidence_id`，渲染前校验 ID 属于输入、文本/数组长度受限、标题非空、无未知字段。校验失败即降级。
5. 对普通寒暄占比高的日期，正文可以简短地说明“未观察到明确的工作推进”，不把问候误写成进展；统计仍保留在“数据口径”中。

## 输出格式与兼容性

保持已有 frontmatter 字段，并新增：

```yaml
generation_mode: llm_curated       # 或 deterministic_fallback
narrative_schema_version: 1
prompt_version: report-curation-v1
coverage_status: complete          # complete / partial / fallback
```

正文统一采用如下阅读层级：

1. `摘要`：2–5 句，直接回答本周期发生了什么。
2. `重点进展/主题`：合并重复会话，不展示 session ID。
3. `关键决策与产出`：没有则省略而非编造。
4. `风险、阻塞与待确认`：区分已观察事实和建议。
5. `下一步`：仅从证据明确支持的后续动作提炼；无依据时写“暂无明确后续动作”。
6. `数据口径与覆盖`：会话数、消息数、日报输入数、缺口恢复、LLM/降级状态。
7. `Evidence References`：保留现有证据清单；Notebook 可默认折叠，固化流程继续使用该区的 URI。

现有 `RhythmReportContent` 和 reader 必须向后兼容：旧报告没有新增 frontmatter 时按现有解析与显示；新报告也始终保留 `summary`，令周/月聚合与旧客户端可读取。`summary` 应改为 LLM 归纳的短摘要而不是“Summarized N sessions”。

## 分阶段实施

### P1：基础契约、配置与报告事实包

- 在 core 定义数据结构、序列化、输入大小限制和 evidence 引用校验；增加与 UI 无关的单元测试。
- 在配置中增加显式开关与预算：`reports.llm_curation.enabled`、模型选择（可继承默认 provider）、总输入/输出 token、超时、语言、fallback 策略。默认启用，且可显式关闭。
- 把 `rhythm.rs` 与月报逻辑的收集代码提取为共享 `ReportFactBundle` builder，保证日/周/月统计和缺口语义不变。

### P2：Provider 适配、结构化生成与安全降级

- 在 providers 添加无工具的 `ReportNarrativeGenerator` 适配，使用 schema/prompt 版本化的 JSON 输出。
- 实现超时、取消、provider 错误、无效 JSON、未知 evidence、超预算与内容为空的统一错误分类。
- 实现确定性 fallback renderer，并在 frontmatter/GUI DTO 中暴露 `generation_mode` 与安全的错误类别；日志不得包含完整会话或密钥。

### P3：接入日报、周报、月报并消除月报双实现

- 日报：从会话 digest 建 bundle，调用 generator，写入归纳正文与完整 evidence。
- 周报：以已生成日报的归纳和缺口 session 为事实输入；不得仅将日报全文再塞入模型。
- 月报：收敛 AutoDream 与 GUI 的重复生成路径；月报仍写 Report-owned 路径，保留 scheduler 的错误标记/重试计数语义。
- 保持 GUI 手动触发 API；必要时扩展返回 DTO 和 Notebook 展示，显示“LLM 归纳”或“确定性降级”，不把模型内部错误暴露给用户。

### P4：评估、回归与发布控制

- 建立固定 fixture：示例中大量问候/工作区浏览的日报应得到简洁、无幻觉的“主要为基础交互，未见实质开发完成”归纳，而非七条会话转录。
- 建立包含实际计划、决策、阻塞和跨日跟进的日/周/月 fixture，人工验收 evidence 引用、去重、中文可读性、无 session ID 泄漏到正文。
- 灰度：先以 feature flag 在开发/测试工作区启用，采集成功率、fallback 率、输出长度、人工有用性反馈；达标后才默认开启。

## 验收标准

- 手动日报、周报、月报在 LLM 可用时正文均为结构化中文归纳，`summary` 不再是纯计数句，且正文不逐条罗列 session ID 或原始聊天片段。
- 每个实质性结论均可追溯到存在的 evidence ID；输入不足时明确说明，不出现虚构项目、完成项、决定或下一步。
- 同一报告输入可在 mock provider 下稳定验证 schema、引用和渲染；真实 provider 不可用时仍原子写入可读 fallback 报告，标识为 `deterministic_fallback`。
- 周/月报正确利用已归纳的下级报告和缺口会话，保留当前 session/token/coverage 统计与路径所有权。
- 旧报告读取、Notebook 列表、预览、固化和手动触发不回归；月报只有一个生产生成实现。
- 验证至少包括：core/autodream/providers/manager（及涉及 GUI 时 GUI）定向测试、`just fmt-check`、`just check`、`just test`，以及 GUI 手动触发 smoke test。环境或既有失败须记录到 `verification.md` 与 `TODOLIST.md`。

## 风险与决策点

- **成本与延迟**：设置每 period 的事实包和输出上限、超时和一次调用上限；不做无界重试。
- **隐私**：最小化发送给 provider 的会话内容；提供关闭 LLM 归纳的配置；审计仅记录元数据与已存在 evidence ID。
- **幻觉**：以“结构化输出 + evidence ID 全量校验 + 不足即说明 + fallback”作为发布门槛，不能仅依赖 prompt。
- **重生成一致性**：保留生成时间、prompt/schema 版本、generation mode；相同 period 覆盖写前提示/确认策略沿用现有 Notebook 行为并补测试。
- **模型/供应商选择**：实施前确认是复用当前默认 provider，还是新增独立 report provider；无论选择何者，native endpoint 不得改写 raw model ID。

## 建议执行顺序

先完成 P1 与 P2 的纯 Rust/Mock 验证，再接入日报，随后周报和月报；最后删除 GUI 中重复的月报生产逻辑。这样可用日报 fixture 固化“去 raw 化、可追溯、可降级”的质量门槛，再向更长周期复用，降低一次性改动三条报表路径的风险。
