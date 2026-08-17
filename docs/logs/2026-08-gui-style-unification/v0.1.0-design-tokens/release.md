# Release: GUI 样式体系统一 v0.1.0

**日期**: 2026-08-17
**分支**: `agent-diva-pro`

## 发布方式

随下一次 `agent-diva-gui` 桌面端常规发布（Tauri 打包）一并发出；本次为纯前端样式层改动，无后端/协议/存储变更，无迁移项。

## 发布前置条件

- [x] `npx vitest run` 通过（435/435）
- [x] `npm run build`（含 vue-tsc）通过
- [ ] 人工 GUI 冒烟（见 acceptance.md）通过后推送

## 回滚方式

- 四个提交均为独立关注点，可按 commit 逐个 revert：`785b954b` → `e8e4b337` → useTheme 提交 → `edbf0134`。
- 整体回滚：`git revert` 上述 4 个提交即可恢复原 `.theme-*` 覆盖层体系；无数据/配置迁移，回滚零风险。
- localStorage 键 `agent-diva-theme` 为新增键，回滚后旧逻辑忽略该键，无副作用。

## 兼容性说明

- 保留的 DEPRECATED gray-utility 兼容桥确保未迁移组件在四主题下表现不变。
- Tailwind 新增 `tk-*` scale 为增量能力，不覆盖默认工具类，现有模板零影响。
