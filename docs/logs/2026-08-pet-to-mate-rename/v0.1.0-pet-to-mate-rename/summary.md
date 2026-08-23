# Summary: pet → mate / 桌宠 → 伙伴 全面改名

## 变更内容

将工作区所有 `pet`/桌宠 相关标识符与文案统一改为 `mate`/伙伴：

| 原形式 | 新形式 |
|---|---|
| `DivaPet` / `divapet` / `diva-pet` | `DivaMate` / `divamate` / `diva-mate` |
| `PetConfig`、`PetSettings`、`DesktopPetOverlay` 等 PascalCase | `MateConfig`、`MateSettings`、`DesktopMateOverlay` |
| Tauri 命令 `pet_*`（13 个） | `mate_*` |
| 窗口标签/事件 `desktop-pet` / `desktop-pet-*` | `desktop-mate` / `desktop-mate-*` |
| HTML 入口 `desktop-pet.html`、`embedded-pet.html` | `desktop-mate.html`、`embedded-mate.html` |
| config.json 顶层键 `"pet"` | `"mate"`（带 `alias = "pet"` 向后兼容） |
| localStorage 键 `agent-diva-pet-config` 等 | `agent-diva-mate-config` 等（一次性迁移） |
| 中文文案 桌宠 / 宠物 / 桌面宠物 | 伙伴 / 桌面伙伴 |
| 英文文案 Pet / Desktop Pet / Diva Pet | Mate / Desktop Mate / Diva Mate |

## 影响范围

- `agent-diva-core/src/config/`：`PetConfig` → `MateConfig`，`Config.mate` 字段
  `#[serde(default, rename = "mate", alias = "pet")]`；loader `normalize_alias_keys`
  新增根级 `pet` → `mate` 合并（避免与默认序列化键冲突产生 duplicate field）。
- `agent-diva-gui/src-tauri/`：`commands.rs` 全部 pet_* 命令与辅助函数、
  `Pet*` 负载类型、`PET_VRM_*` 常量改名；`lib.rs` 命令注册与窗口事件；
  `tauri.conf.json` 窗口 label/title/url；`capabilities/default.json` 窗口列表。
- `agent-diva-gui/src/`：`features/diva-pet/` → `features/diva-mate/`（62 文件），
  组件/服务/工具文件改名，全部标识符、import 路径、CSS 类、Tauri invoke、
  事件名同步；`MateSettings.vue`；`desktop-mate-emotion.ts`。
- `agent-diva-gui/src/features/diva-mate/services/mate-config.ts`：新增
  `migrateLegacyStorageKeys()`，模块加载时一次性把旧 `agent-diva-pet-*`
  localStorage 键复制到新 `agent-diva-mate-*` 键（新键存在时不覆盖）。
- `src/locales/zh.ts`、`en.ts`：locale 键（`pet:` → `mate:`、`openPet` → `openMate`、
  `desktopPet*` → `desktopMate*`）与文案同步更新。
- 磁盘资源目录名（`vrm/`、`voice_resource/` 等真实路径）**保持不变**，
  用户已导入的模型/语音资源不受影响。

## 明确不改

- `docs/dev/archive(old-docs-dont-read-me)/` 全部归档文档。
- `docs/logs/`、`docs/research/` 历史迭代记录与研究快照。
- `docs/engineering/CHANGELOG.md` 已发布历史条目（历史记录保留原名）。
- 无关子串 `snippet`、`sakura-petal`、`anipet-*`（TTS 克隆音色内部前缀）等。

## 备注

- `TODOLIST.md` 中本迭代相关的一行改名由并行会话（Phase4 Bot）在
  `7149cc05` 提交中随其 TODOLIST 整理一并带入（两会话改动重叠于同一行）。
