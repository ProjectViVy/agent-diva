# 性能考量

## 1. 目标

治理不以微优化为目标，但不得因额外层次引入可感知延迟、重复请求或额外 clone。重点关注 chat 热路径、event stream 和大 session。

## 2. 基线指标

Phase 0 记录：

- Manager 收到 chat 到 provider 请求；
- provider 首 delta 到 GUI 首字；
- tool start 到 tool finish；
- session history 读取与映射；
- GUI startup/health ready；
- 100 个 session 的列表与排序；
- 典型 turn 的 clone/serialization 次数。

## 3. 风险

- `TurnSnapshot` 若包含完整 session/context，会产生大 clone；应只持有本轮不可变决议和必要 ID。
- Manager service/handler 分层不得重复 deserialize/serialize `serde_json::Value`。
- Tauri 代理与前端 domain adapter 不能形成二次 JSON stringify/parse 链。
- GUI projection refresh 应按 domain 合并，不在一次 mutation 后触发多个相同 query。
- composable 拆分不能为同一 event 注册多份 listener。
- contract fixture/codegen 不进入运行时。

## 4. 优化边界

- 先测量再优化；
- 使用 `Arc` 共享长生命周期 service，不把整个 `Manager`/`AgentLoop` clone 到每个 request；
- 保持 async 热路径无阻塞文件 IO；
- session/cache 只克隆展示所需字段；
- event reducer 保持幂等，避免用全量 reload 处理每个 token delta；
- 不以 unsafe 换取性能。

## 5. 验收阈值

在同一机器和 fixture 上：

- 首字和 tool roundtrip 的 p95 不得退化超过 10%；
- startup ready 不得退化超过 15%；
- 长 session reload 不得出现数量级退化；
- 常驻 listener/task 数不随 session 切换持续增长；
- 若波动导致无法稳定比较，记录样本和置信区间，不伪造精确结论。
