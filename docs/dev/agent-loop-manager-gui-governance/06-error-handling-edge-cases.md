# 错误处理与边界条件

## 1. 错误分层

| 层 | 错误职责 | 禁止 |
| --- | --- | --- |
| Agent Loop | turn、provider、tool、cancel、persistence 上下文 | 把 denial 当 transport crash |
| Manager service | domain not-found/conflict/invalid-state | 返回无结构字符串 |
| HTTP handler | status + typed error envelope | 吞掉 source 或返回假成功 |
| Tauri adapter | transport/serialization/local-host 错误 | 重做 domain 决策 |
| GUI domain | 可恢复/不可恢复展示与 retry intent | 自行修改 terminal fact |

后续实现应在 Manager 建立统一 `ApiError`，保留当前兼容 response 字段；内部使用 `thiserror`/`anyhow::Context`，非测试代码不新增 `unwrap`/`expect`。

## 2. Agent Loop 边界

- provider stream 在 tool call 参数中断；
- tool 已产生外部副作用但 finish event/persistence 失败；
- cancel 与 tool invocation 同时发生；
- Plan phase 在 iteration 间变化；
- runtime-control 更新工具后 registry 与 policy 不一致；
- memory prefetch/sync 失败；
- token ledger 失败但 session 已保存，或相反；
- session save 原子写失败；
- context compaction 后仍 overflow；
- empty/fallback reply 不得覆盖真实 error。

对 effectful tool 的未知结果不得自动重试。当前系统如果尚无 `OutcomeUnknown` 类型，先保留错误和 audit 证据，不能在结构重构中顺手改变重试政策。

## 3. Manager 边界

- handler 收到超大 body、未知 enum、缺字段；
- runtime 尚未 ready 或正在 shutdown；
- session 不存在、被删除或 revision stale；
- SSE consumer 断开、lag、重复 cursor；
- service 成功但 response serialize 失败；
- embedded 与 external gateway 端口切换；
- background task panic/abort。

Handler 移动后必须保持现有 HTTP status；若当前 status 不合理，记录 TODO，另立行为修复。

## 4. GUI/Tauri 边界

- invoke 成功但 payload shape 漂移；
- Manager command 成功但 projection refresh 失败；
- app startup 时 gateway 暂时未 ready；
- stream 重连后重复事件；
- stop 后迟到 delta/tool finish；
- session cache 比 server 更新；
- approval 卡持有旧 revision；
- LOCAL 文件/资产不存在或路径越界；
- debug 模式没有外部 gateway；
- release embedded gateway 启动失败。

GUI 必须显示“动作可能已提交但刷新失败”，不能把这种情况重试成第二次副作用。

## 5. 可观测性

跨层保留 `trace_id / request_id / session_key / turn_id / tool_call_id`，但不记录 API key、完整 prompt、附件正文和未脱敏工具输出。结构迁移前后应能用相同 ID 串起：

```text
GUI intent -> Tauri/HTTP request -> Manager handler
-> Agent turn -> tool event -> session/plan fact -> GUI projection
```
