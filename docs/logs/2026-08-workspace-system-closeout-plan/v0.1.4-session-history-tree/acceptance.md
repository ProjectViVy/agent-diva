# Acceptance

- 打开历史侧栏可看到“工作区 → channel → 会话”的层次；branch/subagent 等非 root 节点有
  缩进和角色标识，legacy 会话显示为独立历史 root。
- 点击 workspace/channel/session 的折叠控制只改变当前 GUI 展开状态；选择、删除、重命名、
  置顶仍使用原始稳定 session key。
- 搜索子会话时，其 root/父级路径仍在结果中，用户不会看到无上下文的孤立 child。
- 旧 flat session payload 仍能正常展示为 root；无 lineage 字段不会阻塞 GUI。
