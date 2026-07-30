# 验收标准

## 1. 功能

- [ ] 普通 GUI “立即反思”会启动真实 worker，而非只创建 pending run。
- [ ] 真实会话结果被脱敏记录为 evidence，纯聊天或未验证推断不进入长期候选。
- [ ] AutoDream 能生成 0..N 个结构化、非固定模板候选。
- [ ] 每个候选有 evidence、scope、sensitivity、confidence、risk 和 expected value。
- [ ] 候选通过 Laputa proposal，不直接写 typed authority。
- [ ] 批准、编辑后批准、拒绝、并发、重启恢复、回滚全部闭环。
- [ ] applied record 可在新会话 Recall；reverted/tombstoned record 不可 Recall。
- [ ] 用户纠正能驱动修订/supersede proposal。

## 2. 安全与数据完整性

- [ ] AutoDream 无 shell、文件写入、外部 authority 和自批准能力。
- [ ] 日志、metrics、SSE 事件无 Memory/prompt/tool payload。
- [ ] workspace 隔离、receipt 过期/撤销、未知状态均 fail closed。
- [ ] 所有崩溃窗口最多一次 authority 修改。
- [ ] 备份、恢复、manifest rollback 与完整性检查通过。

## 3. 产品可用性

- [ ] 无需调用 API 或编辑配置文件即可完成主流程。
- [ ] 每个失败状态都有用户可理解的原因和下一步。
- [ ] 不可用旧入口已隐藏或删除。
- [ ] GUI、CLI/headless、Manager/Tauri reason code 一致。

## 4. 质量

- [ ] 所有切片聚焦测试通过。
- [ ] `just fmt-check`、`just check`、`just test` 全通过。
- [ ] GUI tests/build、Tauri check、deletion-proof、Rust 1.80 gate 通过。
- [ ] 10k Memory、长会话、并发 run、SSE 重连和 provider 限流基线通过。

## 5. 最终真实桌面

- [ ] 原 G2D 六项全部通过。
- [ ] 第七场景“完成任务 → AutoDream → proposal → apply → 新会话 Recall → rollback 后消失”通过。
- [ ] 使用发布候选二进制、新 profile 与升级 profile 各一次。
- [ ] 验收文档保存脱敏 ID、revision、版本、时间、界面结果和错误日志。

只有以上全部满足，Evolution 才能从“冻结/不可用”标记为“可用”。
