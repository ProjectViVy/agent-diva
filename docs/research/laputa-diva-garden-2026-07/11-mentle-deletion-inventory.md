# 11 — Mentle / MenPalace / memtle 删除清单与 clean-break 验收门

> **RG-RSCH-08a / RG-E8-S4a** · 对应研究问题 **Q26、Q27**
> 权威决策：[`laputa-memory-final-architecture.md`](../../architecture/laputa-memory-final-architecture.md) §Mentle clean break
> 历史 MSRV 证据：[`docs/logs/2026-07-e8-mentle/v0.6.0-dependency-gate/summary.md`](../../logs/2026-07-e8-mentle/v0.6.0-dependency-gate/summary.md)
> 日期：2026-07-24

---

## 1. 执行摘要

**Rust runtime（workspace crates）已无 `memtle` 依赖、无 `mentle` feature、无 Mentle DB reader。** clean-break 在 **编译与执行路径** 上已 Mentle-free。

**产品与技术债仍集中在 GUI 与文档**：设置卡片、config shape、Tauri legacy command、`list_mentle_tools` binding、以及 **242 个文件** 的历史 Mentle 引用（多为 `docs/dev/archive*` 与 pro 迭代日志）。`C:\Users\Administrator\Desktop\morediva\memtle` 为 **独立 sibling 仓库**（crates.io `memtle 0.1.2`，MSRV 1.88），与 deep-governance **无 Cargo 耦合**，但 Garden 仍内嵌 Mentle 作为 MemoryOS 后端（见 `03-garden-contract-audit.md`）— 属 Garden 边界，**非** Diva runtime 接回。

**Q26**：本文件 §3–§5 为完整删除清单。
**Q27**：§6 为删除后 **无兼容残留** 的自动化证明门。

---

## 2. 决策事实（ADR + TODOLIST）

| 事实 | 来源 |
|------|------|
| Mentle/MenPalace **从 Diva 目标架构删除**，无兼容层 | `laputa-memory-final-architecture.md:17-18`, `145-158` |
| 不得抬 MSRV 仅为接 Mentle | ADR §Superseded；`v0.6.0-dependency-gate/summary.md` |
| `RG-E8-S4a` 待执行：删 dependency/feature/adapter/route/DTO/GUI/tests | `TODOLIST.md:389` |
| 旧 `memtle 0.1.2` MSRV 1.88 vs workspace 1.80 | `memtle/Cargo.toml:10`（外部只读扫描） |

---

## 3. Runtime 与 Cargo（Q26 — 可执行代码）

### 3.1 Rust workspace — **已清洁**

| 检查项 | 结果 | 方法 |
|--------|------|------|
| 根 `Cargo.toml` `memtle` 依赖 | **无** | `rg memtle Cargo.toml` |
| 各 crate `Cargo.toml` | **无** | `rg memtle agent-diva-*/Cargo.toml` |
| `features = ["mentle"]` | **无** | 全仓 `.rs`/`.toml`（排除 docs） |
| `mentle_runtime` / `HybridMemoryProvider` | **无** | 无 `agent-diva-core` / `agent-diva-agent` |
| `justfile` Mentle job | **无** | `rg mentle justfile` |
| CI deletion-proof / clean-break | **无 Mentle 特赦** | 脚本目录无 mentle 字符串 |

**结论**：deep-governance **无需** 删除 Rust Mentle 实现（已被 R1 clean-break 移除）；**需** 删除/替换 GUI 与 legacy host 残留。

### 3.2 仍含 Mentle 字符串的 **可执行/可发布** 面对（必须处理）

| # | 路径 | 类型 | 说明 |
|---|------|------|------|
| 1 | `agent-diva-gui/src/components/settings/MentleSettingsCard.vue` | GUI 组件 | 完整 Mentle 设置 UI；引用 `listMentleTools` |
| 2 | `agent-diva-gui/src/api/domains/tools.ts` | API domain | `MentleToolConfigShape`, `listMentleTools` — defer throw |
| 3 | `agent-diva-gui/src/api/index.ts` | Barrel export | 导出 Mentle 类型/函数 |
| 4 | `agent-diva-gui/src/api/v1/binding.ts:98` | Capability | `list_mentle_tools` status **defer** |
| 5 | `agent-diva-gui/src/composables/useAppConfig.ts:40` | Config default | `mentle: { ... }` 默认块 |
| 6 | `agent-diva-gui/src/types/toolsConfig.ts:22` | Type | `ToolsConfig.mentle` |
| 7 | `agent-diva-gui/src-tauri/src/legacy/commands.rs:1138-1140` | Tauri command | `list_mentle_tools` → `reject_legacy_pro_api` |
| 8 | `agent-diva-gui/src/components/SettingsView.stories.ts` | Storybook | fixture `mentle` 字段 |
| 9 | `agent-diva-gui/src/components/NormalMode.test.ts:114` | Test fixture | `mentle` config |
| 10 | `agent-diva-gui/dist/assets/*` | **构建产物** | 含 mentle 字符串 — `cargo clean`/rebuild 后随源码删除消失 |

**注意**：`SettingsView.vue` **未** import `MentleSettingsCard`（`rg MentleSettingsCard agent-diva-gui` 仅命中 card 自身）。卡片为 **orphan 组件**，但仍在仓库中且 dist 已打包。

**i18n**：`general.mentleTitle` 等 key **仅** 在 `MentleSettingsCard.vue` 引用；`locales/zh|en/core.ts` **无** mentle 条目 — 若渲染会 fallback/缺失（进一步证明应删除）。

### 3.3 不含 Mentle 的相关 crate（确认边界）

| Crate | Mentle |
|-------|--------|
| `agent-diva-laputa` | 无 |
| `agent-diva-local` | 无 |
| `agent-diva-manager` / `-local` | 无 |
| `agent-diva-cli` | 无 |
| `agent-diva-execution` | 无 |
| `agent-diva-gui-test` | 未检出 runtime mentle API |

---

## 4. 文档、Backlog、研究（Q26 — 非 runtime）

### 4.1 活跃文档（需更新或标注 superseded）

| 路径 | 动作 |
|------|------|
| `TODOLIST.md` | 保留 `RG-E8-S4a` 直至删除 Done；**6** 处 Mentle 叙述为 **决策/待办** — 正确 |
| `STATUS.md` | **2** 处 — 陈述 Mentle 已决定删除、未执行 — 正确 |
| `docs/architecture/laputa-memory-final-architecture.md` | **Keep** — 权威 clean-break |
| `docs/research/laputa-diva-garden-2026-07/03-garden-contract-audit.md` | **Keep** — Garden 侧 Mentle 事实 |
| `docs/research/laputa-diva-garden-2026-07/12-security-governance-failure-model.md` | **Keep** — Mentle 删除安全含义 |
| `docs/research/deep-governance-reintegration-2026-07/gui-api-governance-inventory.md` | 更新：`list_mentle_tools` → **DROP**（非 defer） |
| `docs/research/agent-diva-hooks-architecture.md` | **1** 处 — 审阅是否仍声称 Mentle 集成 |

### 4.2 历史 / 归档（保留为 archive，不删 Git）

约 **230+** 文件位于：

- `docs/dev/archive(old-docs-dont-read-me)/mentle-integration/**`
- `docs/dev/past/legacy-docs/dev/mentle-integration/**`
- `docs/logs/past/2026-05-mentle-runtime/**`
- `docs/logs/2026-06-mentle-governance-exclusion/**`
- `docs/logs/2026-07-e8-mentle/**`
- `docs/prds/prd-laputa-2026-06-12/**`（FR-6xx Mentle 边界 — **历史 PRD**）

**推荐**：在 `docs/dev/archive(...)/README` 或各目录头增加 **「SUPERSEDED by laputa-memory-final-architecture.md 2026-07-24」** 横幅；**不** 从 Git 删除（ADR：Git history 为 archive）。

### 4.3 全仓 grep 统计（2026-07-24 快照）

| 范围 | 文件数（约） |
|------|-------------|
| 全仓 `mentle\|memtle\|Mentle\|MenPalace` | **242** |
| `agent-diva-*` + `TODOLIST` + `STATUS` + 活跃 architecture/research | **~10**（含 GUI） |
| 排除 `docs/dev/archive*` + `docs/logs/past*` + `docs/dev/past*` | **显著下降**（以 CI gate 为准） |

---

## 5. 外部 `memtle` 仓库历史耦合（只读）

**路径**：`C:\Users\Administrator\Desktop\morediva\memtle`（**非** deep-governance 子模块）

| 事实 | 证据 |
|------|------|
| crates.io 包名 `memtle` 0.1.2 | `memtle/Cargo.toml:2-3` |
| MSRV **1.88** | `Cargo.toml:10` |
| Turso/SQLite 栈、MCP/cli features | `Cargo.toml:12-15`, dependencies |
| 与 Diva **无** workspace path 依赖 | deep `Cargo.toml` 无 entry |
| example 注释提及 `agent-diva-providers` / `agent-diva-nano` | `memtle/example/src/*.rs` — **文档性**，非编译耦合 |
| pro 时代集成契约 | `docs/dev/.../11-s2-a3-published-crate-constraints.md` |

**Diva 与 memtle 的历史关系**：pro 通过 optional feature 嵌入 `memtle 0.1.2`；deep clean-break **从未** 接回。Garden（Go）仍用 Mentle facade 作 canonical memory — **Diva 同步目标为 Garden HTTP/mem_*，非 Diva 内嵌 memtle**（`03-garden-contract-audit.md` §1）。

---

## 6. 删除执行清单（RG-E8-S4a 建议顺序）

### 6.1 P0 — 产品面（用户可见）

1. 删除 `MentleSettingsCard.vue`
2. 从 `useAppConfig.ts` / `toolsConfig.ts` 移除 `mentle` 字段
3. 删除 `api/domains/tools.ts` 中 Mentle 类型与 `listMentleTools`
4. 从 `api/index.ts` 移除 export
5. 从 `binding.ts` 移除 `list_mentle_tools` 行
6. 删除 `legacy/commands.rs` 中 `list_mentle_tools` 及 Tauri 注册
7. 更新 `SettingsView.stories.ts`、`NormalMode.test.ts` fixtures
8. 更新 `gui-api-governance-inventory.md`：`list_mentle_tools` → **CUT**

### 6.2 P1 — 文档诚实

1. `TODOLIST.md`：勾选 `RG-E8-S4a` 子项并指向本 inventory
2. 归档目录加 SUPERSEDED 头（批量脚本或单次 commit）
3. **禁止** 活跃 STATUS/ADR 声称「Mentle 已删除」直至 P0 完成

### 6.3 明确 **不做**

| 动作 | 理由 |
|------|------|
| 删除 `morediva/memtle` sibling 仓库 | 非 Diva workspace；Garden 可能仍参考 |
| 添加 Mentle DB import reader | ADR 禁止 online migration |
| 提高 MSRV 到 1.88 接 memtle | ADR §Superseded |
| 在 GUI 用 Laputa skeleton **假装** Mentle recall | 违反 honest stub |

---

## 7. Q27 — 如何证明无兼容残留（clean-break gates）

### 7.1 推荐 CI gate（新增 `just mentle-clean-break-check` 或扩展现有 `deletion-proof-check`）

```bash
# 1. Workspace 无 memtle 依赖
rg -i "memtle" Cargo.toml agent-diva-*/Cargo.toml && exit 1 || true

# 2. 无 mentle feature / runtime 模块
rg -i "mentle|memtle" --glob "*.rs" --glob "!**/tests/**" agent-diva-* && exit 1 || true

# 3. GUI 产品树无 Mentle 表面（排除 dist — dist 不应提交）
rg -i "mentle|memtle|MentleSettings" agent-diva-gui/src agent-diva-gui/src-tauri/src \
  --glob "!**/dist/**" && exit 1 || true

# 4. Capability registry 无 mentle binding
rg "list_mentle_tools|mentle" agent-diva-gui/src/api/v1/binding.ts && exit 1 || true

# 5. justfile / scripts 无 Mentle CI job
rg -i "mentle|memtle" justfile scripts/ && exit 1 || true

# 6. 活跃 docs 无「Mentle 作为 Diva backend」宣称（allowlist ADR/TODO/inventory）
rg -i "mentle.*backend|embed.*mentle|features.*mentle" docs/architecture docs/research/laputa-diva-garden-2026-07 \
  --glob "!**/11-mentle-deletion-inventory.md" && exit 1 || true
```

### 7.2 运行时证明

| 证明 | 方法 |
|------|------|
| 不读 Mentle DB | 无 `memtle`/`turso` 依赖 → 物理不可打开 Mentle 格式库 |
| 无 legacy `/api/tools/mentle/*` | Tauri `reject_legacy_pro_api` 已 block；删除 command 后 grep 零 |
| MSRV 1.80 默认 build | `just ci` 不启用 1.88 toolchain |
| fail-closed | `listMentleTools` 已 throw — 删除后 API 不存在 |

### 7.3 数据迁移边界

- **在线**：runtime **永不** 读取 pro-era Mentle 数据库或 `.mentle` 路径
- **离线（可选）**：仅 `agent-diva-migrate` / Memory Pack（`10-memory-pack-and-migration.md`）显式 import → Laputa record model；**非** E8-S4a 范围

### 7.4 被否决的「证明」方式

| 方式 | 理由 |
|------|------|
| 仅人工声明「已删除」 | 违反 TRUTH_IN_CODE |
| 保留 feature flag `mentle=off` | ADR 禁止 feature flag 兼容 |
| 只删 GUI 不删 binding | grep gate 仍失败 |

---

## 8. 研究问题逐条回答

### Q26 — Mentle 删除 **完整清单**是什么？

**答**：

1. **Runtime**：已空；无 Cargo 动作除确认性 grep。
2. **必须删/改**：§3.2 共 **10** 项 GUI/API/Tauri/test（+ dist rebuild）。
3. **文档**：§4 — 活跃文档更新 + 归档标注；**242** 历史文件保留 archive。
4. **外部**：`morediva/memtle` 与 Garden Mentle **不在** Diva S4a 删除范围，但 Diva **不得** 再依赖。

### Q27 — 删除后如何 **证明** 无兼容残留？

**答**：§7 六条自动化 gate + `just ci` + 可选 GUI unit test「settings 无 mentle 字段」+ CLI smoke 无 mentle 子命令。验收标准：**活跃源码 grep 零 Mentle**（allowlist 仅 ADR、本 inventory、历史 archive 路径）。与 `deletion-proof-check` / `clean-break-boundary-check` 合并维护。

---

## 9. 风险与开放假设

| 风险 | 缓解 |
|------|------|
| 用户 pro 配置 JSON 仍含 `mentle` 键 | migrate 或 schema 忽略未知键 + GUI 不渲染 |
| dist 提交含 mentle 字符串 | `.gitignore` dist 或 CI 不扫描 dist |
| Garden 文档读者混淆 Diva/Garden Mentle | `03` 明确 Garden 仍用 Mentle；Diva 用 Laputa-Diva |
| 归档 doc 被误作实现依据 | SUPERSEDED 头 + `TRUTH_IN_CODE` |

**开放假设**：

- [ ] 是否在 E8-S4a 同时删除 pro 配置 schema 中的 `mentle` 字段定义（若 deep 仍有拷贝）
- [ ] `gui-api-governance-inventory` 中 `list_mentle_tools` 标为 CUT 还是 DEFER 直至 Laputa recall 替代 UX 设计

---

## 10. 推荐结论

1. **立即** 执行 §6.1 GUI/API 删除 — 低风险、无 runtime 依赖。
2. **新增** §7.1 grep gate 进 `just ci` — 防止 Mentle 回归。
3. **不要** 接回 `memtle` crate；Laputa-Diva 用自研轻量存储（`04`）。
4. 归档文档 **保留** Git，加 superseded 标记即可。

---

*扫描方法：`rg -l "mentle|memtle|Mentle|MenPalace"` 全仓；`memtle/Cargo.toml` 外部只读；未修改任何 runtime 代码。*
