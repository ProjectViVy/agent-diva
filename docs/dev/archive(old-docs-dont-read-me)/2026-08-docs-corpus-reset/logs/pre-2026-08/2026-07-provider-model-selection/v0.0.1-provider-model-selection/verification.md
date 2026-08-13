# 验证记录

- `npm test -- --run src/components/settings/ProvidersSettings.test.ts`：通过。
- `npm run build`：通过；Vite 仅报告既有的大包体积警告。

回归测试验证点击模型卡片会立即调用配置保存动作，并发出更新后的快捷模型列表。
