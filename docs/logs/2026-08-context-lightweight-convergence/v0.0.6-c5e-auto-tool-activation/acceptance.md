# Acceptance

1. 模型调用 `tool_search` 后，下一次 provider 请求的 tool schema 直接包含命中的 deferred tool。
2. provider 可以立即调用该工具；不需要第二个 mount 操作。
3. 搜索未授权工具不会把它加入 active set；来源下线后不会执行。
4. 新搜索替换旧 active set，激活项最多 8 个；下一用户 turn 不继承旧激活。
5. session JSONL 不新增 discovered/mounted/revision metadata，重启按空 active set 开始。
