# Channel settings recovery release

日期：2026-09-04

## 发布方式

本轮只做本地源码集成：功能在隔离分支 `feat/gui-channel-recovery` 完成，完成门禁后以
no-ff 方式合入本地 `dev`。没有 push、远程发布、安装包构建或生产配置变更。

主要提交组：

- C6-D 纳入交付链：`1a195be9`。
- 依赖审计与 pnpm policy：`42828d22`、`82c834b5`、`dbc02460`。
- GUI/API 恢复与状态竞态修复：`185982f1`、`263f35da`、`ee132057`、`816ffcb7`、
  `6af0f4a6`。
- probe/runtime/Manager 事务所有权：`bab45995`、`8c1ea72e`、`dbe70847`、`bd62a095`、
  `5fc842ad`、`53b3dded`。
- 浏览器烟测跟进：`2a167e58`、`46402c92`。

## 兼容与迁移

- `channels.removed` 是可反序列化缺省为空集的 additive 配置字段；删除固定频道时写入
  tombstone，后续重新配置会移除对应 tombstone。
- GUI 构建链只支持仓库固定的 pnpm 10.34.5 与 `pnpm-lock.yaml`；`package-lock.json`
  不再是恢复来源。
- Manager/Tauri 的新 probe/delete 命令保持固定频道 allowlist，不接受任意 adapter 名称。

## 回滚

回滚应按提交组逆序进行，并同时回滚 GUI 调用面、Tauri bridge、Manager API 和 Core 配置
字段，不能只回滚页面。若回滚依赖策略，需要明确接受双 lockfile/旧漏洞图重新出现；该状态
不属于安全等价版本。已写入配置的 `channels.removed` 在旧版本读取时会被 serde 忽略，但
恢复到旧 UI 前应先备份 profile 配置。
