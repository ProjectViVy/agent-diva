# CHANNEL-EPIC C2a 发布说明

C2a 仅提交到隔离 `feat/channel-epic` 分支，不合并 `dev`、不推送、不部署。新 Fabric Kernel
没有接入产品 bootstrap，旧频道运行路径保持不变。C6 完成全链路迁移和 Clean Break 后再原子合入。

回滚方式是回退 C2a 单一提交；由于没有配置、数据迁移、journal 或运行时接线，不需要数据回滚。
