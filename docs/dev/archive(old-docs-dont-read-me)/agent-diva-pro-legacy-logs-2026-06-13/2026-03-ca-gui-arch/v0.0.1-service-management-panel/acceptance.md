# CA-GUI-ARCH ServiceManagementPanel 验收步骤

## 验收清单（对照 ui-ca-gui-arch-service-management-panel.md 第 8 节）

- [x] 安装按钮在已安装状态下禁用（设计 4.1 节）
- [x] `just ci` 通过
- [x] 主题 theme-default、theme-dark、theme-love 下 styles.css 已覆盖 ServiceManagementPanel 所用类
- [x] verification.md 已创建并记录 smoke 步骤

## 用户视角验收（需手动执行）

1. **开发模式**：`cd agent-diva-gui && pnpm tauri dev`，进入设置 → 通用，确认「服务管理（仅打包应用可用）」占位可见
2. **打包模式**：`bundle:prepare` + `pnpm tauri build`，安装后进入设置 → 通用，确认服务管理面板完整展示（状态卡片、刷新按钮、安装/启动/停止/卸载按钮）
3. **Windows/Linux**：可点击安装/启动/停止/卸载，状态正确更新
4. **macOS**：显示受控降级提示，操作按钮 disabled
5. **800px 宽度**：布局正常，按钮组可换行
6. **中英文切换**：所有文案均有 i18n key
