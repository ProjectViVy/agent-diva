# E1B Session evidence 离线回填

Migration CLI 增加 `experience dry-run | apply | rollback`。它从既有 session 的结构化
tool-result 元数据生成 Experience evidence，只使用 tool name、call ID、时间与错误
分类；完整输出只用于判断 `Error:` 前缀，不会进入 Journal、manifest 或报告。

dry-run 不创建文件。apply 先持久化 prepared manifest，再幂等追加，最后标记 applied；
进程中断后重放可补齐。rollback 只删除 manifest 本次新增的 ID，保留 apply 前已存在
的相同 evidence。容量不足时 fail closed，不驱逐在线证据。
