# 频道能力参考实现对照：验收标准

用户/产品侧验收本次研究时，应确认：

1. 能明确回答“为什么主对照选 Octos，而不是 ZeroClaw/OpenFang”；
2. 能看到 QQ、DingTalk、Feishu 的逐频道差异，而不是只比较频道数量；
3. 能区分结构参考、能力标杆和 Bridge/TCK 参考，避免全盘复制单一项目；
4. 能在 `TODOLIST.md` 找到待正式立项的 `CHANNEL-CAPABILITY-CONTRACT-EPIC`；
5. 能确认本次未修改生产 channel 代码，也未把 Octos 的 Rust 2024/MSRV 1.85 引入
   agent-diva 当前 Rust 1.80 约束。

验收入口：

- [`docs/research/channel-capability-reference-2026-08/README.md`](../../../research/channel-capability-reference-2026-08/README.md)
- [`TODOLIST.md`](../../../../TODOLIST.md)
