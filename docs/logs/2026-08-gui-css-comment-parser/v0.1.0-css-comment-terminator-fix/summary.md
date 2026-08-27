# CSS 注释终止符修复

## 变更

- 修复 `agent-diva-gui/src/styles.css` 兼容桥说明文字中意外出现的 `*/`。
- 注释内容改为分别描述 `text-gray-*` 与 `bg-white`，不改变任何选择器或主题行为。

## 影响

修复 Vite/PostCSS 在解析全局样式时报告 `Unknown word themeMode` 并中断 GUI 启动的问题。
