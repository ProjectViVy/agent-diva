# C1c Workspace Memory Epoch Acceptance

## 用户/产品验收

1. Typed BML/Laputa apply 成功后，不出现聊天消息或 Session 历史事件。
2. 同工作区已有 Session 在下一次 Prompt 组装时看到最新 Memory；未被访问的 Session
   不被主动遍历重建。
3. 同一 authority revision 的幂等重放不重复渲染或递增 Prompt projection revision。
4. 不同 workspace 的 Runtime 不消费本次刷新命令；WM、checkpoint、历史和工具状态保持不变。
5. apply 恢复与 rollback 走同一通知出口；AgentLoop 重启后仍从当前 BML revision 正确启动。

## 完成定义

C1c 外部 L1 apply 通知在定向测试、workspace Clippy、全量测试、CI 门禁和最小可执行
smoke 全部通过后完成；Skills reload 继续作为独立后续项。
