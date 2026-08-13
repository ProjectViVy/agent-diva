# 验收

1. dry-run 返回稳定 migration ID、workspace ID、记录 digest 与计数，且无磁盘写入。
2. apply 写 prepared/applied manifest，并仅追加结构完整的 tool-result evidence。
3. 相同输入重复 apply 不增加记录且返回同一 migration ID。
4. rollback 只移除本 migration 新增的 evidence。
5. 缺失 call ID/tool name、容量超限、manifest 冲突和重复 rollback 均 fail closed。
6. Journal 和 manifest 不含历史工具输出。

E1 至此具备在线新增与旧 session 可逆回填两条完整路径。
