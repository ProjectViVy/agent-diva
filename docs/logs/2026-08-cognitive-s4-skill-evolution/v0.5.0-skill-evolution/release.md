# I1-S4 发布说明

本轮未发布、未推送，也未创建 PR。所有变更仅提交到本地 `agent-diva-pro`。

发布前必须先完成 `EVOLUTION-S4-DESKTOP-SMOKE`，随后按 S5 删除无调用的旧 Evolution
符号，并在 S6 执行最终 clean-break 证明。数据策略为不迁移、不双读：旧
`workspace/skills` 仅留在原处，不进入生产权威。

回滚可按本轮五个关注点提交逆序执行；机器 Skill Home 数据不应由代码回滚自动删除。
