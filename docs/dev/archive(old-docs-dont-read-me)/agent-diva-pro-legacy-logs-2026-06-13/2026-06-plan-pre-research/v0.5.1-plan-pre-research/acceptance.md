# Acceptance

## v0.5.1-plan-pre-implementation-research

## Acceptance Criteria 对照

### 交班要求

| 要求 | 状态 | 证据 |
|------|------|------|
| planning 类型和 store 放哪个 crate | ✅ | pre-implementation-research.md §Code Map: `agent-diva-core::planning` |
| SQLite 是否适合第一期 | ✅ | §Store: 确认适合，复用 SqliteIndex 模式 |
| 初始化/迁移怎么做 | ✅ | §Store: CREATE TABLE IF NOT EXISTS + foreign_keys(true)，第二期引入正式 migration |
| AgentLoop/context/tool/manager 的最小接入点 | ✅ | §Code Map 汇总表 + §Runtime（5 hook 点 + 具体行号） |
| 第一批 PR 应该切成哪几块 | ✅ | §First PR Slice: PR #1（Types + Store ~300行）+ PR #2~#6 建议顺序 |
| 第一批测试怎么写 | ✅ | §Test Plan: 9 项测试覆盖单元/集成/恢复 |

### 明确不做

| 明确不做 | 状态 | 证据 |
|------|------|------|
| Kanban | ✅ 未涉及 | §Risks: "本次延后" |
| 多 worker | ✅ 未涉及 | §Tool Surface: 单 in_progress constraint |
| dispatcher | ✅ 未涉及 | §Risks: "无 dispatcher" |
| 自动复杂度判断 | ✅ 未涉及 | 全文无相关内容 |

### GUI 补充

| 要求 | 状态 | 证据 |
|------|------|------|
| 组件树设计 | ✅ | `planning-gui-design-supplement.md` §3 |
| 状态颜色与图标 | ✅ | §7 全 GUI 统一映射表 |
| Tauri 后端命令 | ✅ | §5 3 个新 invoke + DTOs |
| 导航入口 | ✅ | §3 NormalMode.vue 侧边栏扩展 |
| 实现优先级 | ✅ | §8 Phase A/B/C |
| 国际化字符串 | ✅ | §6 zh.ts/en.ts 补充块 |

### 可直接开工的第一 PR checklist

- [x] `agent-diva-core/src/planning/` 目录结构已定义（§Code Map）
- [x] SQLite schema 已确定（§Store，5 表 + 外键）
- [x] 初始化代码模式已确定（§Store 伪代码）
- [x] 测试策略已定义（§Test Plan）
- [x] Cargo.toml 无需变更（sqlx 已在 workspace 级别声明）

## 用户/产品视角

预调研文档达到以下可用性标准：
- 开发同事可以直接按 §First PR Slice 的 checklist 开工
- 无需额外上下文即可理解接入点和改动范围
- 所有风险已标注控制措施和升级信号

## 签字

调研负责人: Hermes Agent
日期: 2026-06-01
