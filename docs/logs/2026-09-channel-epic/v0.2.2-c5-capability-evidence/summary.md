# C5-V/C5-Q：能力证据与 QQ 纵向验证

日期：2026-09-02  
分支：`feat/channel-epic`  
交接基线：`03385c8d`  
Octos 参考：`5ea987813de4fd2afdd1d78f2106ad2868f0d923`

## 交付内容

- 六个 native adapter 均补充了 adapter-owned wire fixture/mock、请求与响应断言、typed
  receipt/error 和生命周期测试，并新增六份 `platforms/*-gate3.md` 审计页。
- 共享 TCK 冻结六频道 capability snapshot，检查 representative false capability 在
  transport 前失败，解析六频道 fixture，并机器校验 29 行
  `capability-evidence.json`。
- QQ 本地官方形状 mock 覆盖 token、gateway、C2C、群 @、admission-before-dedup、
  heartbeat、C2C/group outbound routing、`msg_seq`、reply `msg_id` 和真实响应 ID。
- 新增仓库外凭据驱动的 `#[ignore]` QQ live harness；凭据缺失时不发起网络请求，也不把
  本地 wire 证据冒充真机证据。
- 已处理频道 all-target clippy 的 test-only 债务；没有修改公共协议、频道配置、全局
  endpoint、Manager 生产装配或 C6 clean break。

## 未关闭项

`C5-V` 与 `C5-Q` 保持未勾选。QQ D-013（官方 intents 投递证据）和 D-014（官方媒体
wire path）仍为 `blocked`；真实 QQ 凭据/平台权限未注入，因此 live smoke 未完成。所有
不具备完整 fixture、测试、请求、响应和 receipt/error 的能力仍保留为 `partial`，详见
[`evidence-manifest.md`](../../../dev/channel-epic/c5-octos-migration/evidence-manifest.md)。

## 主要提交

- `92cd9257` Telegram wire contract
- `9945a231` Discord Gateway/REST contract
- `39ded197` QQ wire contract
- `b527f1e6` Feishu token contract
- `a0150837` DingTalk wire contract
- `0c77d9db` Email transport contract
- `8ba1d3b3` shared capability TCK
- `168f3bb2` QQ live harness
- `c6b0a756` channel all-target clippy cleanup
- `67ba5cb2` Gate 3 evidence manifest and audit pages
