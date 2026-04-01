# agent-diva-gui 测试说明

## Story 4.1 — Person 单一叙事（FR8 / FR9）

自动化回归见 Vitest：`src/components/personNarrativeRegression.spec.ts`。

- **用户可见根 `user-visible-app-root`（`NormalMode` 整壳）**：默认聊天 Tab 下在根内断言 `person-agent-conversation-stream` **恰好 1 个**，且含唯一 transcript / composer；侧栏进入 **神经** 后根内 **0** 个对话流壳；神经 → 再回 **聊天** 后仍为 **1** 个。
- **路径 A（皮层关）与路径 B（皮层开）**：在隔离 `ChatView` 中同样断言单壳；**皮层开 + Tauri** 下在 `NormalMode` 整壳中桩 `__TAURI_INTERNALS__`，验证出现 `cortex-toggle` 时仍 **不会** 增加第二套对话流壳。
- **神经视图（隔离挂载）**：断言 **不存在** `person-agent-conversation-stream`（内部协作不以第二套并列聊天室形式暴露）。

**手动走查清单与验收表格**（路径 A/B 勾选表）见仓库根下产品故事文件：

`_bmad-output/implementation-artifacts/4-1-person-narrative-regression.md`

运行 GUI 单元测试：

```bash
npm test
```
