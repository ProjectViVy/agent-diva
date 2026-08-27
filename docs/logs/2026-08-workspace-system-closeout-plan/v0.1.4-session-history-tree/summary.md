# WS-05：历史会话分层 GUI

本阶段把 ConversationSidebar 从平铺列表改为只读层级投影：

- 顶层显示当前 workspace header，下面按 channel 分组。
- root、branch、subagent、ephemeral 根据 session lineage 以缩进树行展示；legacy 会话明确
  标识为历史 root，不产生伪造父子关系。
- 折叠/展开只影响本地投影，不改变服务端 session 集合；置顶继续是会话属性，不复制为
  第二套列表。
- 搜索命中 child 时自动保留其父级路径，active session 仍以稳定 `session_key` 定位。
- App/ChatView/NormalMode 透传后端 lineage 字段与当前 workspace root；消息详情没有迁移到
  列表，仍由主聊天区域负责。
