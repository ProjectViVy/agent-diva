# Release / handoff

本轮没有发布、推送、合并到 `dev` 或切换 Manager 生产装配。C6 Clean Break、原子切换和
退役频道删除均未执行。

所有 QQ 真实凭据只允许由仓库外环境变量注入：

```text
AGENT_DIVA_LIVE_QQ_APP_ID
AGENT_DIVA_LIVE_QQ_SECRET
AGENT_DIVA_LIVE_QQ_C2C_OPENID
AGENT_DIVA_LIVE_QQ_GROUP_OPENID
```

可回滚范围是本轮提交；优先按提交粒度回退，不使用 destructive worktree 命令。当前
工作树的并行锁在最终交接前保持，由 Lead 释放。
