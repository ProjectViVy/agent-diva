# CHANNEL-EPIC C2b 发布说明

C2b 仅提交到隔离 `feat/channel-epic` 分支，不合并 `dev`、不推送、不部署。新 Adapter Runtime
目前由 TCK/fake adapter 驱动，产品仍使用旧频道运行路径。

回滚方式是回退 C2b 单一提交；本批没有配置、凭据、数据库、journal 或外部平台副作用。
C3 继续在同一隔离分支开发 Neuro-Link Gateway、Projection Journal 和 Service Catalog。
