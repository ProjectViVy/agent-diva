# Channel settings recovery summary

日期：2026-09-04

## 结果

频道设置页的两处占位能力已经恢复为生产路径：向导可对未保存的候选凭据执行真实原生
`ProbeHealth`，卡片删除会通过 Manager/Tauri API 重置凭据、禁用频道并写入
`channels.removed` tombstone，刷新后不再展示已删除频道。连接失败不会强迫用户丢弃输入；
用户可在明确失败提示后选择继续保存。

GUI 依赖审计修复独立记录在相邻的
[`v0.4.11-gui-audit-remediation`](../v0.4.11-gui-audit-remediation/)；本轮最终交付同时包含其
pnpm-only、单 lockfile、精确安全版本和持续 CI audit 门禁。

## 主要实现

- Core/Manager/Tauri 建立候选 probe、runtime status 与 delete 合同；删除使用固定六频道
  allowlist，错误响应只暴露稳定 code/静态 message。
- GUI 将 load、单频道 status refresh、test、save、delete 分成独立 operation state，并以
  generation/token 丢弃 stale result；删除和保存不能重复提交，弹窗关闭不会晚到改写状态。
- probe、Email blocking work、supervisor stop cleanup 均有全局/单频道硬并发上限；公共
  waiter 被取消后，独立 owner 仍持有 native future、JoinHandle 与 semaphore permit。
- ChannelRuntime 的 reconfigure/shutdown 与 Manager 的 runtime+ConfigLoader mutation 都由
  独立 owner 完整收尾。正常 Manager actor 等待内部 completion，继续串行其他配置写；
  actor/client 被取消也不会把磁盘配置和 live runtime 留在不同 generation。
- 浏览器烟测发现并修复频道刷新按钮缺失翻译，以及 `ChatView` 未注册 Lucide `X` 图标。

## 影响范围

涉及 `agent-diva-core` 频道配置、`agent-diva-channels` probe/runtime、
`agent-diva-manager` HTTP/事务、`agent-diva-gui` Tauri bridge/频道页面，以及 GUI 依赖策略。
未改动真实平台凭据、远程环境或生产部署。

## 保留项

C5/C6-E 的真实平台入站、最终 receipt、切换后桌面断线恢复和 broad 全工作区 MSRV 仍是
开放验收；本轮没有凭据，不以 mock/Tauri 单测替代真实平台证明。其余非阻断 P2/P3 已写入
根 `TODOLIST.md`。
