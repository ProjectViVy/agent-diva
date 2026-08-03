# Goal 阶段停点与续作协议

## 通用停点报告

每个停点必须报告：

- 阶段和完成/未完成 story；
- 精确 commit SHA 与 staged/unstaged 状态；
- 生产行为变化和保持的兼容契约；
- 测试命令、结果、真实 smoke 观察；
- 新增/关闭 TODO、剩余风险、回滚方式；
- 是否使用 profile/key/provider/network/admin 权限（预期全部否，隔离 profile 除外）；
- 下一阶段将修改的 scope。

Goal 报告后进入等待。只有用户明确回复“继续”或给出修订方向才可续作；不得把沉默、
测试通过或自动 continuation 当成人工批准。

## 门禁 A：GMH-30B2

人工确认：

- Plan/Memory 重启恢复 Pending；悬空 Allowed 撤销并建立新 Pending；
- prepared+Consumed 仅幂等完成原 operation；
- Plan execution/Memory apply 无重复副作用；
- 三域确实共享唯一 ledger/coordinator composition；
- domain payload 未进入 ledger。

## 门禁 B：GMH-31

人工确认：

- `ApprovalView`、decision body、reason code 和 HTTP 映射；
- SSE 事件名、cursor、顺序、reconnect 行为；
- 旧 HTTP/Tauri/SSE contract tests 通过；
- 新 service 是读写统一入口，但不是第二 authority。

## 门禁 C：GMH-32

先完成自动化，再由用户在隔离桌面观察并确认：

- badge/抽屉/三域就地卡一致；
- risk/evidence/diff/grant/TTL/stale/expired 文案可理解；
- edit-and-approve、断线重连、重复事件和 refresh failure 行为；
- keyboard/focus/非颜色状态表达。

## 门禁 D：GMH-33

人工确认：

- interactive CLI 无隐式 allow；
- headless 默认 fail-closed，退出码/JSON 稳定；
- queue 仅显式启用且返回可查询 request；
- Command 不因 queue 持久化 raw payload；
- shell/Plan/Memory E2E 均符合预期。

## 最终门禁

- Windows release EXE 可从隔离产物启动；若涉及安全策略，附人工授权记录；
- debug/release 桌面 smoke、全量 tests/build/clippy 全绿；
- M3/TODO/logs/commits/rollback 完整；
- 无真实 provider/key/profile/push/system-policy 越权；
- 用户明确接受 M3 后，Goal 才能标记 complete。

## 修订与阻塞

- 用户改变产品决策：先更新本资料包并单独提交，再 `/goal edit`；
- 新问题当期可修则同 story 修复，否则写 `TODOLIST.md`；
- 连续遇到同一外部阻塞时报告精确证据，不以受限验收冒充 M3 完成；
- 需要真实 provider 或系统安全策略时暂停，不能用 Goal 的持续性推断授权。
