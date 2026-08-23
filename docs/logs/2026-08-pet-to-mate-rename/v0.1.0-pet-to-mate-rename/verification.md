# Verification: pet → mate 全面改名

## 执行命令与结果

| 步骤 | 命令 | 结果 |
|---|---|---|
| Rust 格式化 | `cargo fmt` + `just fmt-check` | 通过（无 diff） |
| Rust lint | `just check`（clippy -D warnings，全工作区） | 通过 |
| Rust 全量编译 | `cargo check -p agent-diva-gui`（连带全工作区依赖） | 通过 |
| Core 配置测试 | `cargo test -p agent-diva-core --lib config::` | 70 passed，含新增 `test_load_migrates_legacy_pet_section_to_mate` |
| 工作区测试 | `just test` | 仅 6 个预存在失败（`agent-diva-cli` wiremock 502，`CLI-WIREMOCK-502-PREEXISTING`，LOCK.md 多次记录，与本次改动无关）；其余 crate 全绿 |
| 排除 CLI 复测 | `cargo test --workspace --exclude agent-diva-cli` | 全绿（见提交说明） |
| 前端单测 | `cd agent-diva-gui && npm test`（vitest run） | 68 files / 487 tests 全部通过，含新增 2 个 localStorage 迁移用例 |
| 前端类型检查 | `npx vue-tsc --noEmit` | 通过（exit 0） |

## 残留检查

对全仓库执行 `diva[-_]?pet|DivaPet|divapet|桌宠|宠物|pet`（边界感知）复查，
活跃代码零残留；剩余命中均为有意保留：

- `mate-config.ts` / `mate-config.test.ts`：legacy 键迁移常量与用例（必须引用旧键名）。
- `agent-diva-core` schema/loader：serde `alias = "pet"` 与 loader 合并规则。
- `docs/dev/archive(old-docs-dont-read-me)/`、`docs/logs/`、`docs/research/`、
  `docs/engineering/CHANGELOG.md` 历史条目：历史记录按策略保留原名。
- 无关子串：`snippet`（session 搜索）、`sakura-petal`（CSS 装饰）、
  `anipet-`（TTS 克隆音色内部命名前缀，外部约束）。

## 兼容性验证

- 旧 `config.json`（顶层 `"pet"` 节）经 `ConfigLoader::load()` 正确合并进
  `Config.mate`，序列化输出键为 `"mate"`（单测
  `test_load_migrates_legacy_pet_section_to_mate` 断言）。
- 旧 localStorage 键 `agent-diva-pet-config` / `*-asr-default-enabled-migrated-v1`
  一次性迁移到新键；新键已存在时不覆盖（vitest 2 用例）。
- Tauri `load_config` 走 `ConfigLoader::load`（coalesce 后输出 `mate`），
  `save_config` 直接 serde 反序列化（`alias = "pet"` 兜底）。

## GUI 冒烟

见 `acceptance.md`；启动 GUI 后确认侧边栏"伙伴"入口、设置页开关、
桌面伙伴弹窗与配置持久化（`config.json` 出现 `"mate"` 节）。
