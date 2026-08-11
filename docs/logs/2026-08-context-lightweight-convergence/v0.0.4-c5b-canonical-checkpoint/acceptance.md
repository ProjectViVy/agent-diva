# Acceptance

1. 连续多次压缩后 session 只有一个 `canonical_checkpoint`，正文不超过 8,000 字符。
2. 第二次压缩把旧 checkpoint 与新增安全裁剪前缀合并，并替换旧 checkpoint；模型只
   看到一个 checkpoint system block。
3. Auto、Manual、Reactive 使用相同安全边界和渲染顺序，触发类型只用于诊断。
4. 完成工具链只以工具名/状态/artifact ref 折叠；未完成或缺结果工具组保持完整 active
   配对，不能切成孤儿消息。
5. Reactive 失败不推进 durable index、不覆盖旧 checkpoint；成功后 retry 使用 pending
   视图，finalize 保存当前 turn 后原子提交。
6. reset/delete 后不再注入 checkpoint，普通重启从新字段恢复；旧压缩字段按未知字段
   忽略。
7. `rg` 删除证明阻止旧压缩字段、MetaCompactor、多摘要 marker 和双执行入口回流。
