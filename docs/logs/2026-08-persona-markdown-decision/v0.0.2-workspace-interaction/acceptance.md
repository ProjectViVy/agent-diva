# Acceptance

本次决策记录满足以下产品验收条件：

1. 左侧文件导航保留，永久右侧栏删除，中央工作区只有当前文档、待审变更和历史三态。
2. 当前文档使用 Markdown 源码与可折叠预览；用户保存不走审批。
3. 待审变更以只读 before/after 文本 Diff 展示，只允许接受或拒绝。
4. 接受不拆成 approve/apply；base revision 失配后必须 stale，禁止覆盖新人类编辑。
5. Persona 内容审查不进入聊天页 Approval Center，也不复用 Evolution/Memory Governance。
6. 历史版本不可原位编辑；载入只替换本地草稿，用户保存后才产生新的当前版本。
7. 第一版不做逐段接受、提案内编辑、暂缓或自动三方合并。
8. 记录仍保持破坏性 clean break，不引入旧人格 JSON/Markdown 兼容层。
