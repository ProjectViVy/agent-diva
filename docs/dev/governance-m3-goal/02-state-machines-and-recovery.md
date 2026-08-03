# 状态机、消费与重启恢复

## 统一状态

唯一 durable approval 状态为：`Pending`、`Allowed`、`Denied`、`Revoked`、
`Consumed`、`Expired`。所有 mutation 使用 request ID、expected version、幂等键和
canonical digest；首个有效 CAS 响应获胜。

domain payload 不属于 ledger：Command 仅存在于 waiter；Plan revision 和 Memory proposal
由各自 store 持久化。Manager detail projection 只在读路径组合两者。

## 三域恢复矩阵

| 重启前状态 | Command | Plan | Memory |
|---|---|---|---|
| Pending | 撤销，不恢复 waiter/正文 | 恢复同一 Pending | 恢复同一 Pending |
| Allowed，无 prepared record | 撤销 | 撤销并为当前 revision 建新 Pending | 撤销并为当前 proposal digest 建新 Pending |
| Allowed，有 prepared record | 撤销 | consume 成功后才可继续初始化 | consume 成功后才可 apply/recover |
| Consumed + prepared | 不适用 | 幂等完成同一 execution 初始化 | 按 journal 幂等完成/确认同一 apply |
| Denied/Revoked/Expired | 终态 | 终态；domain 保持未执行 | 终态；proposal 保持未应用 |
| Consumed，无 prepared | 终态且告警 | 标记 recovery inconsistency，禁止新执行 | 标记 recovery inconsistency，禁止新 apply |

恢复按 core 分页接口扫描当前 workspace，只处理匹配 capability。恢复不广播旧
requested 事件；恢复结果产生新的 durable revoke/request/update 事件供当前客户端查询。

## Plan 执行初始化

1. Plan revision 进入 AwaitingApproval，创建 `PlanExecute`、High、Once request；
2. Allow/Deny 只写 governance ledger，不直接进入 Execute；
3. Allow 后校验 revision/hash/policy/resource/expiry；
4. 在 canonical planning store 建立唯一 prepared execution context，绑定 request ID、
   receipt version 和 revision hash；
5. consume Once receipt；消费失败时 context 不得 Ready；
6. 以 planning CAS 把 plan 转为 Execute，并把同一 context 标记 Ready；
7. 重试返回同一 execution ID；不得创建第二 context 或重复 materialize TODO；
8. prepared 后失败可由启动恢复完成；无 prepared 的 Allowed 必须撤销并重批。

## Memory apply

1. proposal digest 变化立即撤销旧 Pending/Allowed，建立新 request；
2. 高风险和非 Low proposal 仅允许 Once；
3. Allow 后创建/复用绑定 proposal digest、request ID/version 的 prepared apply journal；
4. consume receipt 成功后才可进入 typed apply；
5. apply/changelog 写入按现有 journal 幂等恢复；
6. 重试返回已有 changelog，不重复写 Memory；
7. Allowed 但无 journal 时撤销并为当前 digest 建新 Pending；
8. 缺失目标、schema conflict、digest mismatch 均 fail-closed。

## 取消、超时与并发

- scope/session stop：Pending/Allowed 写 Revoked；Plan/Memory domain payload 保留；
- deadline：显式写 Expired；不得只依赖 UI countdown；
- Allow/Deny/cancel 多客户端竞争：version CAS 首胜，其余返回 typed conflict；
- 相同幂等键+相同 operation 返回已提交状态；同键不同 operation 返回 conflict；
- side effect outcome unknown 不自动 retry，只记录 correlation 和人工恢复指引。
