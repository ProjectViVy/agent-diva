# Manager HTTP、SSE 与 Tauri 契约

## 统一读模型

`ApprovalView` 固定包含：

- `request_id`、`version`、`domain`（command/plan/memory）、`capability`；
- `resource`、`risk`、`status`、`created_at`、`expires_at`；
- payload-free `subject`、`evidence`、`receipt`、`reason_code`；
- `actions`（allow/deny/cancel/edit/apply 等可用 intent）；
- 可选 `presentation`，由 domain assembler 提供安全标题、diff/摘要和来源会话。

`presentation` 不进入 ledger。Command 进程重启后没有 presentation，也不会保持 Pending。
Plan/Memory presentation 来自 canonical domain store，并再次校验 digest。

## 新增接口

| Method/path | 行为 |
|---|---|
| `GET /api/approvals` | 按 domain/status/session 分页列出，稳定 request-id cursor |
| `GET /api/approvals/:request_id` | 返回 ledger 状态与安全 domain presentation |
| `POST /api/approvals/:request_id/decisions` | 提交 expected_version、idempotency_key、decision、grant |
| `POST /api/approvals/:request_id/cancel` | 使用 expected_version、idempotency_key 撤销 |
| `GET /api/approvals/events` | 使用 cursor 重连统一事件流 |

Plan 编辑继续使用 revision API，Memory 编辑继续使用 proposal API。编辑成功必须撤销旧
request 并返回/触发新 Pending；不设计能接受任意 domain payload 的通用 edit body。

## 兼容适配

- 保留当前 Command、Plan report、Laputa path/method/status/JSON 字段；
- 旧 mutation handler 委托统一 service，并在缺少新字段时生成 request-scoped 幂等键，
  但不得绕过 ledger CAS；
- 新 GUI/CLI 使用统一接口；旧 Tauri command 作为薄兼容适配器，到 GMH-53 才评估删除；
- 不新增 `/v1` alias，不维护第二份状态，不让客户端猜新旧 schema。

## Typed reason code

至少冻结：

`approval_not_found`、`approval_version_conflict`、`approval_idempotency_conflict`、
`approval_expired`、`approval_already_resolved`、`approval_already_consumed`、
`approval_digest_mismatch`、`approval_invalid_grant`、`approval_invalid_transition`、
`approval_payload_unavailable`、`approval_persistence_failed`、
`approval_required_noninteractive`、`approval_queue_unavailable`、
`approval_outcome_unknown`。

HTTP 映射固定：not-found→404；version/idempotency/already/expired→409；invalid body/grant/
transition/digest→422；queue/runtime unavailable→503；persistence→500。旧 endpoint 的已冻结
status 不合理时先兼容，另以 header/body reason 暴露准确代码，不在 M3 偷改旧契约。

## SSE 序列

统一事件名：`approval.requested`、`approval.updated`、`approval.resolved`。每条包含
`event_id/cursor`、request ID、version、domain、status、reason、occurred_at 和 correlation。

- cursor 指向 durable ledger event，不依赖进程内 broadcast 序号；
- reconnect 从 cursor 之后重放，有界分页，事件 ID 去重；
- 同 request version 只产生一个 committed fact；
- requested 必须先于 resolved；recovery revoke/new request 保持各自版本顺序；
- SSE lag/serialize/disconnect 不改变 domain state；
- 旧 command requested SSE 和 Agent bus Plan event 保持兼容，由统一事件投影产生。

## Tauri

新增统一 list/detail/decide/cancel/start-stream bridge；Tauri 只做 transport、serialization 和
本地连接生命周期，不推导审批状态。Rust fixture 必须被 TypeScript guard 解析；旧 command
和 Plan Tauri command 名/payload 原样保留并委托新 bridge。
