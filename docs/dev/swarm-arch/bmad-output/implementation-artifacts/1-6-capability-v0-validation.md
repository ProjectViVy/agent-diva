---
story_key: 1-6-capability-v0-validation
story_id: "1.6"
epic: 1
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - agent-diva/agent-diva-agent/project-context.md
---

# Story 1.6：能力声明 v0 校验（核心）

Status: done

## Story

As a **系统**,  
I want **解析并校验 v0 能力 manifest（最少字段）**,  
So that **FR11 在服务端成立且错误可向上返回**。

## Acceptance Criteria

1. **Given** 合法与非法 manifest 样例（含 JSON 语法错、缺必填、类型错、重复 id 等）  
   **When** 加载并校验  
   **Then** 非法样例返回 **可机读错误**（结构化：错误码/层级/路径/人类可读消息）；合法样例进入 **占位注册表**（进程内、可查询条数或列表摘要，不要求持久化或热重载全量能力）

2. **And** 校验逻辑与 **FR11**、架构 **CapabilityRegistry / manifest v0 子集** 对齐；实现落点以 **`agent-diva-agent` 的 skills/能力路径** 为起点，必要时新增 **registry 子模块**（可仍在同 crate 内），与 `architecture.md`「能力与 manifest（FR10–11）」映射一致

3. **And** **不阻塞** Epic 1 其他故事：无强制要求在本 story 接 Tauri/GUI，但错误与成功结果的 **序列化形态** 须 **预留 Epic 4 Story 4.2 消费契约**（见下文「Epic 4 UI 衔接」）

4. **And** 至少 **一组** 无 GUI 测试：`cargo test` 覆盖合法通过 + 若干非法用例断言机读错误字段（与 `architecture.md` 数据架构节「能力 manifest v0 子集 + golden files 方向」一致时可补充 golden JSON）

## Tasks / Subtasks

- [x] **冻结 v0 manifest 最小字段集**（AC: #1–#2）  
  - [x] 在实现或 `docs` 片段中写明 **必填键、可选键、类型、id 唯一性**；与头脑风暴/PRD 中 **priority、本地 manifest、冲突覆盖** 等 v0 共识 **不矛盾**（具体键名以实现冻结为准）  
  - [x] 使用 **serde** 反序列化 + 显式校验层（缺字段、越界 priority、空 id 等）

- [x] **解析与校验 API**（AC: #1）  
  - [x] 入口：从 **字节/字符串/路径** 解析 JSON（或文档化单一格式）  
  - [x] 输出：`Result<ValidatedManifest, CapabilityManifestErrors>`（或等价）；**多错误** 时建议 **Vec** 一次性返回，便于 UI 列表展示（Epic 4）

- [x] **可机读错误模型**（AC: #1、#3）  
  - [x] 每条错误至少包含：**`code`（稳定枚举或字符串常量）**、**`message`（中文或 i18n key 由上层决定，本层可提供默认英文/中性键）**、**`location`（`file` | `field` 维度 + 可选 JSON Pointer / 点路径）**  
  - [x] **serde** 可序列化为 JSON，供后续 Tauri `invoke` 直接透传或 thin map（与 NFR-I2 白名单字段演进时加 `schema_version`）

- [x] **占位注册表**（AC: #1）  
  - [x] 合法校验通过后：**注册** 为 `Vec`/`HashMap` 或轻量 `CapabilityRegistry` stub：**按 capability id 索引**，可查询 **数量、id 列表、摘要**  
  - [x] **明确非目标**：本 story **不** 实现动态下载、签名 manifest、与工具链完整绑定（留待后续）；占位须 **线程安全策略** 在注释或 ADR 中一句话说明（`Arc`/`RwLock` 或仅初始化期单线程）

- [x] **与现有 SkillsLoader 边界**（AC: #2）  
  - [x] 阅读 `agent-diva-agent`：`SkillsLoader` 与 **SKILL.md** 路径；能力 **manifest** 与 **单技能文件** 的关系在注释中写清：**v0 manifest 声明「包/能力条目」**，解析可与 loader 协同或并列模块，**避免** 在 `agent-diva-meta` 中实现（遵守 ADR-A：编排 crate 不拉 meta；本 story 主要在 agent 侧）

- [x] **验证**（AC: #1–#4）  
  - [x] `cargo test -p agent-diva-agent`（或实际承载 crate）覆盖校验与注册表  
  - [x] `cargo clippy -p <crate> -- -D warnings`

## Dev Notes

### FR11 与架构要点

| 主题 | 要求 | 来源 |
|------|------|------|
| FR11 | 加载并校验用户能力声明（**v0 字段子集**），错误时可见反馈（本 story 落实 **服务端/库侧** 校验与机读错误） | `prd.md` |
| 数据验证 | 能力 manifest **v0 子集**；校验错误对 **用户/日志** 可见；协议 JSON **golden files** 可与 `DESIGN_SUPPLEMENT` §8 方向对齐 | `architecture.md` — Data Architecture |
| 代码落点 | **`agent-diva-agent` skills 路径 + 新 registry 子模块（若拆出）** | `architecture.md` — Requirements to Structure Mapping |
| 现有技能栈 | `SkillsLoader`、工作区与内置 `SKILL.md`（YAML frontmatter + JSON 元数据） | `agent-diva-agent/project-context.md` |

### 可机读错误（约定）

- **目标**：网关、CLI、后续 Tauri 层 **无需解析自由文本** 即可区分 *文件级*（整份 JSON 无法解析）与 *字段级*（某键缺失/类型错误）。  
- **建议字段（最小）**：`code`、`severity`（可选）、`message`、`location_kind`（`file` \| `field`）、`path`（可选，如 `/capabilities/0/id`）。  
- **稳定性**：`code` 为 **契约**；`message` 可改文案而不破坏前端逻辑（Epic 4 可用 code + path 做定位）。

### 占位注册表（Placeholder registry）

- **职责**：校验通过后的 **内存态** 登记，供本 Epic 后续故事或 **doctor**（Story 4.4）拉 **数量/摘要**。  
- **非职责**：不与完整 `agent-diva-capability` 长期设计（见 `ARCHITECTURE_DESIGN.md`）一次性对齐；**显式标注** `// v0 placeholder`。  
- **测试**：至少断言「合法 manifest → `register` 后 `len` 或 `get(id)` 符合预期」。

### Epic 4 UI 衔接（Story 4.2 依赖本故事）

- **依赖关系**：`epics.md` 写明 **Story 4.2 依赖 1.6**；4.2 AC 要求校验失败时 **字段级或文件级** 明确错误，成功时列表/状态与 **1.6 一致**。  
- **本 story 交付边界**：实现 **Rust 侧** 校验 + 机读错误类型 + 占位表；**不** 要求完成 Vue/设置页。  
- **衔接方式**：将错误与注册摘要类型设计为 **`serde::Serialize`**，并在 Dev Notes 或 crate 文档中列出 **推荐 Tauri DTO 字段名**（与现有 `invoke` 命名惯例一致，见 `architecture.md` API Naming）。  
- **UX**：`ux-design-specification.md` — 能力 manifest（FR10/11）错误为 **字段级或文件级**；与 4.2 对齐。

### 禁止与范围

- 不在本 story 实现完整蜂群调度、工具白名单执行、或 GUI。  
- 不引入 **swarm → meta** 依赖；校验逻辑放在 **agent（及允许的 core 扩展）** 边界内。  
- 不因等待 Epic 4 而阻塞合并：先满足 **无 GUI 测试** 与 **稳定错误类型**。

### Project Structure Notes

- 工作区根：`d:\newspace\agent-diva\`（以本机为准）。  
- 规划产物：`d:\newspace\_bmad-output\planning-artifacts\`。

### Testing Requirements

- **单元测试**：合法/非法样例 + 多错误聚合（若实现）。  
- **无需 E2E**；Epic 4.2 再补 UI。

### References

- `epics.md` — Story 1.6、Epic 4 Story 4.2、Coverage / 故事依赖  
- `architecture.md` — FR10–FR11、Data Architecture、Structure Mapping  
- `prd.md` — FR10、FR11  
- `ux-design-specification.md` — manifest 错误粒度  
- `agent-diva-agent/project-context.md` — SkillsLoader、模块边界  

## Dev Agent Record

### Agent Model Used

Cursor 内联 Composer（实现 Story 1.6）

### Debug Log References

（无）

### Completion Notes List

- 在 `agent-diva-agent` 新增 `capability` 模块：`error`（稳定 `code` + `location_kind` + 可选 `path`）、`validate`（JSON → `Value` 后显式 v0 校验，多错误聚合）、`registry`（`PlaceholderCapabilityRegistry` + `RwLock<HashMap>`，注册全有或全无）。
- v0 字段：`capabilities[]` 必填；每项必填非空唯一 `id`；可选 `name`、`description`、`priority`（0–1000）；根级可选 `schema_version`，若存在须为 `"0"`。
- `skills.rs` 模块文档说明与 `SKILL.md` 加载的职责边界。
- `cargo test -p agent-diva-agent`、`cargo clippy -p agent-diva-agent -- -D warnings` 已通过。

### Change Log

- 2026-03-30：实现能力 manifest v0 校验、机读错误 DTO、占位注册表与单元测试（Story 1.6）。
- 2026-03-31：代码审查闭环 — `INVALID_UTF8` 错误码、模块文档、`capability` 入库提交；`DUPLICATE_ID` 保持共用。

### File List

- `agent-diva/agent-diva-agent/src/capability/mod.rs`（新增）
- `agent-diva/agent-diva-agent/src/capability/error.rs`（新增）
- `agent-diva/agent-diva-agent/src/capability/validate.rs`（新增）
- `agent-diva/agent-diva-agent/src/capability/registry.rs`（新增）
- `agent-diva/agent-diva-agent/src/lib.rs`（`pub mod capability`）
- `agent-diva/agent-diva-agent/src/skills.rs`（模块文档：manifest vs SKILL.md）
- `_bmad-output/implementation-artifacts/sprint-status.yaml`（1-6 → review）
- `_bmad-output/implementation-artifacts/1-6-capability-v0-validation.md`（本文件状态与任务勾选）

### Review Findings

_Code review（BMAD workflow）2026-03-31；`review_mode`: full。_

- [x] [Review][Decision] **DUPLICATE_ID 是否拆分** — **已决议（2026-03-31）：** 保持 manifest 内重复与 `register` 冲突共用 `capability.manifest.duplicate_id`；Epic 4.2 若需区分再以契约版本演进。

- [x] [Review][Patch] **未跟踪源码需入库** — **已处理：** `git add agent-diva-agent/src/capability` + `lib.rs` / `skills.rs` 中与 1.6 相关的导出与文档，并已提交（未纳入同分支上其它 `agent_loop` 未提交改动）。

- [x] [Review][Patch] **非 UTF-8 与 JSON 解析错误码** — **已处理：** 新增 `codes::INVALID_UTF8`（`capability.manifest.invalid_utf8`），`from_bytes` 使用该机读码；新增 `invalid_utf8_is_distinct_from_json_parse` 测试；`mod.rs` 补充须通过验证 API 构造的说明。

- [x] [Review][Defer] **`Deserialize` 绕过校验入口** — `ValidatedManifest` / `ValidatedCapability` 带 `Deserialize`，调用方若直接反序列化可跳过 `parse_and_validate_*`。v0 占位可接受；建议在 `capability::mod` 或类型上注明「须通过验证 API 构造」。 [`validate.rs`] — deferred, pre-existing design choice for serde 对称性

- [x] [Review][Defer] **Poisoned `RwLock` 时查询静默** — `len` / `ids` / `summary` 在锁中毒时返回 0 或空列表，无错误传播。占位 registry 可接受；生产化时可改为 `Result` 或文档约束。 [`registry.rs` 约 62–83 行] — deferred, v0 placeholder

---

_Context: Ultimate BMad Method story context — 与 `1-1-swarm-crate-workspace.md` 模板对齐；面向 Story 1.6 / FR11 v0 manifest 校验。_
