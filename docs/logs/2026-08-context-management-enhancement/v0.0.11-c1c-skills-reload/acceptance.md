# C1c Skills Reload Acceptance

## 用户/产品验收

1. Manager 上传或删除 workspace skill 成功后，不出现聊天消息；同 workspace 已有 Session
   在下一轮 Prompt 组装时看到技能目录变化。
2. 同一技能重复上传完全相同内容时，不产生 reload；失败上传、删除不存在技能和删除
   内置技能也不产生 reload。
3. `memory_distill` 新建技能后，其他同 workspace Session 的下一轮 Prompt 能看到该技能；
   覆盖已有技能的 proposal 或失败结果不提前刷新。
4. 不同 workspace 的 reload command 被忽略；Session history、Memory、Frozen Core、Mask
   和无关工具状态保持不变。

## 完成定义

C1c Skills reload 在定向测试、workspace 检查、全量测试、CI 门禁和最小 CLI smoke 全部
通过后完成；后续主线回到 AutoDream → Laputa proposal → typed Memory → Recall 反馈。
