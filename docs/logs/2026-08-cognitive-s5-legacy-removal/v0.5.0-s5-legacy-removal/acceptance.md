# S5 验收步骤（用户/产品视角）

S5 为拆除切片，验收以「旧面消失 + 新面不受损」为准。自动化证据见
`verification.md`；以下为可在真机执行的确认步骤：

## 1. CLI 冒烟

```
cargo run -q -p agent-diva-cli -- --help
```

- usage 正常输出；无 persona-retire 子命令残留。

## 2. Manager HTTP 面

启动本地 gateway 后：

- 旧治理 URI 全部 404（样例：`/api/laputa/sections`、`/api/laputa/proposals`、
  `/api/laputa/governance/*`、`/api/bml/*` 等已被路由测试固化的 12 个 URI）。
- `/api/approvals` 命令/计划审批仍可用（未删域）。
- `/api/memory/records` BML 读写正常（MemoryHome 权威）。
- `/api/laputa/recall-feedback` 仍可用（保留的唯一 laputa 路由）。
- health 面板 memory 分量反映 `MemoryHome::warmup`，不再出现旧治理降级文案。

## 3. GUI 桌面

- Persona 工作区：无 JSON 编辑门、无永久右栏治理、左栏仅七份文档。
- Memory 工作区：BML 列表/详情直改，无审批按钮，无混域 Inbox。
- Evolution 工作区：仅 Skill 待审（接受/拒绝），无 Memory/人格混箱。
- 全局搜索旧组件（JSON editor、Governance 右栏、Inbox）不可达。

## 4. AutoDream 降级语义

- 未配置机器级 MemoryHome 时手动 run 直接失败闭合，错误信息为
  `AutoDream requires the machine-wide MemoryHome authority`，不落任何
  `.laputa/cognitive`/`proposals` 产物。

## 5. 扫描门（S6 交付后）

- `just cognitive-clean-break-check` 对 D4 §3.1 扫描族在生产路径零命中。

## 遗留

- 真实桌面视觉/交互验收仍在 UI-S2/S3/S4 各自 `*-DESKTOP-SMOKE` 条目跟踪
  （当前执行环境无原生 WebView 控制器）。
- S6（证明门 + 桌面 smoke）未开始。
