# acceptance：沙箱设置保存修复

## 前置

- 构建并启动开发环境：仓库根目录执行 `just make-diva`
  （窗口 1：gateway；窗口 2：GUI `pnpm tauri dev`）。

## 验收步骤

1. 打开 GUI 设置 → "沙箱设置"页：
   - 页面正常加载，"沙箱模式"/"审批策略"下拉框有正确选中的当前值与中文/英文标签。
2. 切换"沙箱模式"（如改为"只读模式"）与"审批策略"，出现重启 gateway 警告横幅。
3. 点击"保存设置"：
   - 期望：成功 toast"设置已保存"（不再出现"保存失败"）。
   - 检查 `~/.agent-diva/config.json` 的 `sandbox` 段，`mode`/`approval_policy`
     为 snake_case 值（如 `read_only`、`on_failure`）。
4. 重新打开"沙箱设置"页，回显第 3 步保存的值，下拉标签正常。
5. 边界：清空"执行超时（秒）"输入框后保存，应成功且 timeout 写为 60。
6. 边界（可选）：临时把 `~/.agent-diva/config.json` 改为非法 JSON，打开页面再保存，
   失败 toast 应附带具体错误信息；恢复文件后保存成功。
7. 重启 gateway 后新沙箱模式生效（原有提示行为不变）。

## 通过标准

步骤 3-5 全部通过；任何失败 toast 必须包含后端真实错误信息而非仅"保存失败"。
