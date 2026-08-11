# Verification

## 命令与结果

- `npx vitest run src/components/ChatView.test.ts` → 6 passed（含新增持久化恢复/写回）。
- `npx vue-tsc --noEmit` → 通过（无类型错误）。
- rust 侧无改动，无需 cargo 验证。

## 覆盖点

| 场景 | 结果 |
|---|---|
| 从 localStorage 恢复 `permissionMode`（trusted） | ok |
| 变更 `permissionMode` 后写回 localStorage（cautious） | ok |
| vue-tsc 类型检查 | ok |
| node_modules 不被 git 跟踪（junction 本地环境） | ok |

## 说明

- 测试环境使用与主工作树共享的 node_modules junction（本地环境，非提交物）。
- 真实桌面 smoke 属后续人工验收项。