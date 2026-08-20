# v0.2.1 已安装技能页删除与 Evolution 托管区分 — Summary

## 背景

用户报告:商店安装的技能在设置页“已安装”标签显示为“:evolution 管理”。根因是
`InstalledSkillsTab.vue` 中 commit de5dc35f 引入的硬编码占位按钮
`<ShieldCheck/>Evolution 管理`——每条技能都挂着,未 i18n、无点击行为,视觉上被误读为
技能名的一部分。后端返回的技能名始终是干净 slug,数据层无误。

用户确认的产品逻辑:

- 市场安装 / 手动上传的技能:可在“已安装”页直接删除,不显示“Evolution 管理”。
- 只有 Evolution 生成的技能:才需要 Evolution 的复杂管理(保留 i18n 提示)。

## 改动

### agent-diva-core(`src/evolution/skill_home.rs`)

- `SkillSummary` 新增 `evolution_managed: bool`(`#[serde(default)]`)。
- 判据:Home 技能存在状态为 `Accepted` 的提案即视为 Evolution 托管。新增无锁辅助
  `accepted_slugs()`(基于 `list_requests_unlocked`),避免与持写锁调用 `read()` 的
  `accept_request`/`update` 路径死锁。
- 不改磁盘格式、不动 frontmatter、不破坏 CAS 哈希语义;对存量进化技能追溯生效。
- 新增单测 `evolution_managed_reflects_accepted_proposals`。

### agent-diva-manager(`src/skill_service.rs`)

- `SkillDto` 增加 `evolution_managed`(`#[serde(default)]`)并从 summary 透传。
- 删除复用既有 `DELETE /api/skills/:slug` + `SkillService::delete_skill`,无新路由。

### agent-diva-gui

- `src-tauri/src/commands.rs`:Tauri `SkillDto` 增加 `evolution_managed`
  (`#[serde(default)]`,兼容旧网关)。
- `src/api/desktop.ts`:`SkillDto` 增加可选 `evolution_managed`。
- `src/components/settings/InstalledSkillsTab.vue`:
  - 移除硬编码占位按钮;`evolution_managed` 技能显示 i18n 提示(盾牌图标,不可点击)。
  - 其余 Home 技能显示真实删除按钮(`skills-btn-danger`):`appConfirm` 确认 →
    `deleteSkill(slug, content_hash)` → toast → 刷新列表(模式同 EvolutionView)。
  - builtin 技能不显示任何操作。
  - 硬编码中文提示行改为 i18n 键 `skillsInstalledHint`。
- i18n 新键(i18n.ts 中文 fallback + locales/en.ts):`deletingSkill`、
  `deleteSkillConfirmTitle`、`deleteSkillConfirmBody`、`skillDeletedToast`、
  `skillManagedByEvolution`、`skillsInstalledHint`(`deleteSkill` 复用既有键)。
- `styles.css` 新增 `.skills-btn-danger`。
- 新增 `InstalledSkillsTab.test.ts`(5 个用例)。

## 影响范围

- 网关 `GET /api/skills` 响应新增字段 `evolution_managed`(向后兼容,旧 GUI 忽略)。
- 已安装技能页行为变更:安装类技能可直接删除(硬删除,同名内置技能恢复可见)。
- 不影响 Evolution 视图既有管理链路。
