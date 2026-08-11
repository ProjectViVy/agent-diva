# C1c Skills Reload Summary

## 完成内容

- 新增工作区隔离的 `ReloadWorkspaceSkills` Runtime Control command；AgentLoop 校验
  canonical workspace ID 后，统一标记当前 workspace 内所有已缓存 Session 的
  `AgentRulesAndSkills` section。
- ContextBuilder 的 reload 保持惰性：通知只写 pending invalidation，下一次 Prompt
  组装才重新读取技能目录；不触碰 Session history、Memory、Frozen Core、Mask 或用户
  可见消息。
- Manager skill upload/delete 接入通知；上传服务比较安装前后的技能目录，真实变更才
  通知，完全相同的重复上传 no-op 不通知，失败操作不通知。
- `memory_distill` 在新建技能并返回 `Applied` 时走同一技能失效标记路径；已有技能的
  `ProposalCreated`、失败和参数错误不触发 reload。

## 影响范围

- 仅影响当前进程中同一 workspace 的 AgentLoop SessionStable cache。
- 不新增配置、数据库 schema 或文件监听；不改变 BML/Laputa authority 写入边界。
