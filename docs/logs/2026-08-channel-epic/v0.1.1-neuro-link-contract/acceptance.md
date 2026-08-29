# CHANNEL-EPIC C1 验收

## 产品/架构验收

- [x] Neuro-Link v1 有单一版本化 JSON Schema，Rust 与 TypeScript fixture 共用它。
- [x] JSON-RPC 2.0、精确版本和未知字段策略已可由正反 fixture 验证。
- [x] typed envelope、地址、关联、内容块、payload、receipt、cursor 和 error 合同已公开。
- [x] hello、service/list、session/open、turn/start、turn/cancel、event/ack、state/resume
      均有 schema fixture 覆盖。
- [x] 旧 pipe、身份伪造、坏扩展和 malformed content 会被 schema/TCK 拒绝。
- [x] 旧 MessageBus、allowlist、loopback guard 的当前行为已有 characterization 基线。
- [x] 五个 Fabric lane 的容量已冻结为代码常量，并明确不复制 Octos 4096。
- [x] 没有新增兼容层、shim、旧 DTO 别名、身份授权字段或第二实时链路。
- [x] GUI 现有页面行为保持不变；C4 才接入 Neuro-Link v1。

## C1 关闭条件

`just fmt-check`、`just check`、`just test`、GUI 全量 Vitest/build、协议 TCK、频道表征和容量
基准均通过后，C1 可作为 `feat/channel-epic` 的独立批次提交；`dev` 继续等待 C6 原子合入。
