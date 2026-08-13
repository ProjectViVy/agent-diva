# 发布

不单独发布、不 push。该 schema 为内部 v1，旧 workspace 没有 Journal 时按空输入处理，
不会影响正常对话或 typed Memory authority。

回滚本提交会停止新增证据；`.agent-diva/autodream/experience/events.jsonl` 是非 authority
衍生证据，可保留供诊断，也可在离线备份后移除。
