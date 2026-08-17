# Verification: GUI 样式体系统一 v0.1.0

**日期**: 2026-08-17
**环境**: Windows 10 (x64), Node v24.3.0, pnpm, agent-diva-gui

## 自动化验证

| 命令 | 结果 |
| --- | --- |
| `npx vitest run`（agent-diva-gui） | ✅ 60 文件 / 435 用例全部通过（含 useTheme 5 个新用例） |
| `npm run build`（= `vue-tsc --noEmit && vite build`） | ✅ 类型检查 + 产物构建通过 |

### useTheme 新用例覆盖点
1. 默认 love 主题且 `data-theme` 应用到 `<html>`。
2. `setTheme` 切换并持久化到 localStorage。
3. 非法主题 id 被忽略。
4. 已持久化主题在模块加载时恢复。
5. localStorage 中非法值回退默认 love。

## 零回归保障手段

- Step 3 退役覆盖层时，先按主题提取每个被删选择器的精确像素/色值写入令牌，再改基类为 `var(--token, fallback)`；发现并纠正过一处色值漂移（兼容桥误用 `--panel-solid` 替代 #111827，已回退为精确字面量）。
- Step 4 仅替换与令牌值完全一致的硬编码点位；深色调与身份色板保留字面量。

## GUI 冒烟（人工，待补）

自动化无法覆盖真实窗口渲染，人工冒烟步骤记录于 `acceptance.md`。建议执行：

```bash
just start   # 或 cd agent-diva-gui && npm run tauri dev
```

观察点：四主题切换即时生效且刷新后保持；聊天区/输入框/头像/气泡/空态/子视图无可见色差；侧边栏删除按钮与右键菜单危险项颜色正常。

## 已知环境性波动

vitest 首跑偶发全量失败（pet-config 中 tauri `invoke` 未定义），重跑即恢复；与本次改动无关，已在会话内复测确认。
