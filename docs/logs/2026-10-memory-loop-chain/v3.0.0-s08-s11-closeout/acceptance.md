# 增量开发验收记录

| 检查 | 结果 | 证据与边界 |
|---|---|---|
| 跨进程新会话召回 | Pass | S08 实际 App 进程重启后，profile 召回事实进入新会话模型请求；三 profile、六项负对照通过。见 `raw/s08-s09-final-suite.jsonl`。 |
| 修正后输入使用新版本 | Pass | 模型请求包含新事实、不含旧事实；assistant 回答与修正内容一致。 |
| 删除后旧事实不回到搜索 | Pass | 真实 tombstone 后重启；重投旧 run 三次；公开搜索不返回已删除 record。 |
| 删除后新会话不召回旧事实 | Pass | 新会话请求/回答不含旧事实、修正事实或删除 record 的上下文标记。 |
| 重放不改写 canonical 来源 | Pass | 原 ingestion、capture sequence、record ID、revision、来源 hash/body 与 canonical 数量保持不变。 |
| 普通构建 hostile-memory 数据边界 | Pass | 真实 ContextHost 候选进入用户上下文；不进入 system authority；Mission、policy、effect receipts 未变；查询 168ms/140ms。 |
| Race hostile-memory 注入边界 | Pass，测试预算 5s | race-only 夹具预算下真实查询完成（3044ms/2000ms），authority/effects 不变。该结果只证明注入边界，不能代表默认预算 race 正向召回通过。 |
| Race 默认 750ms 超时退化 | Pass | 默认预算下实际 source 退化，无候选泄漏；错误保持归一化 `material read failed`。 |
| S11 失败根因可观察性 | Pass | 未导出的 error cause 支持 `errors.Is(context.DeadlineExceeded)`；wire JSON 不包含 deadline 原因且 message 未变。 |
| V19 自动上下文分区与显式工具读取审计 | Open | S08 的剩余边界，未由这批用例覆盖。 |
| Backend 故障恢复与 workspace A/B | Open | S11 全矩阵尚未运行；当前 product identity/workspace 隔离结论仍有限。 |
| 正式 Story / 同候选验收 | Open | S01–S03 正式 gate 未通过；不可据此把 S08/S09/S11 标为 Done。 |

以上是本地开发证据，不是正式 conformance、sealed candidate 或发布证明。
