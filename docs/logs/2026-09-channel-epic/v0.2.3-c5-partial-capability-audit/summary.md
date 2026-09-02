# C5-V：21 条 partial capability 审计

日期：2026-09-02
分支：`feat/channel-epic`
审计基线：`2b3ef682`
Octos 参考：`5ea987813de4fd2afdd1d78f2106ad2868f0d923`

## 目标

本轮只审计 `evidence-manifest.md` 中的 21 条 `partial` 行。六个频道 agent 分别对照
固定 SHA 的 Octos 源码、当前 DIVA adapter、fixture、测试源码和既有验证记录，输出
逐行的 source/protocol/evidence/disposition 结论。

本轮不修改 adapter 行为、fixture、测试实现、公共契约或 Cargo 文件；不编译、不运行
测试、不访问真实平台。各频道只更新自己的 Gate3 审计页，Lead 汇总共享 manifest、JSON
和本轮验证记录。

## 并行任务

Telegram TG-02～TG-06；Discord DC-01～DC-03、DC-05；Feishu FS-01、FS-03～FS-05；
DingTalk DT-01、DT-03～DT-04；Email EM-02～EM-04；QQ QQ-01～QQ-02。

## 阶段结论

六个 agent 已完成固定 SHA 对照，三组交叉复核也确认主判断成立。21 条 `partial` 全部
保持 `partial`，没有一条因“源码存在”或历史测试记录而升级；全量清单仍为 7 条
`verified`、21 条 `partial`、1 条 `blocked/unsupported`。Gate3 页面、manifest 和 JSON
均保留相同 ID/状态。

审计提交为：Telegram `fc1f5a40`、Discord `1a9e1065`、Feishu `e1009c40`、DingTalk
`d9fc146d`、Email `c2851602`、QQ `0c5d2b8e`。每页均记录了当前 DIVA symbol、固定
Octos 源码对照、已有 fixture/test 证据、精确请求或 typed 结果，以及逐项 disposition。

本轮确认的代码层缺口包括：Telegram 群响应门控与超限下载语义；Discord DM/mention/
dedup/下载/reply 语义及 403 分类；Feishu 删除响应 JSON 错误码；DingTalk 主动 heartbeat、
multipart token refresh/idempotency；Email `imap_use_ssl`、MIME 校验和 health/mark-seen
语义；QQ 重连时 heartbeat 状态复位和 invalid-session cooldown。其余未闭合项属于缺少
逐类 wire/lifecycle transcript 的证据缺口，均已在 manifest 和各 Gate3 页展开。

Octos 的 webhook、共享 media/dedup helper 只作为固定参考，未被当作 DIVA 另一条 transport
的实现或证据。D-013/D-014 仍保持 blocked；QQ live smoke 仍需仓库外凭据和平台权限。
本轮不修改 adapter、fixture、测试、公共契约或 Cargo，不编译、不跑测试、不访问外网，
也不合并 `dev` 或进入 C6。
