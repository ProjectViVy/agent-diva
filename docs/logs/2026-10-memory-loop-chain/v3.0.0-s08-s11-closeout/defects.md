# 缺陷、诊断与未完成项

## 已定位并通过边界验证：race 下 native recall source 超时

- **触发条件：** Go `-race` 构建中的实际 App hostile-memory recall；Mentle hybrid card search。
- **症状：** 默认 ContextHost 源预算为 750ms，source 返回归一化 `material read failed`，真实候选没有进入模型输入。
- **根因证据：** `errors.Is` 经 Garden 归一化错误链识别 `context.DeadlineExceeded`；受控测量显示 card search race 下约 2.3s，而 evidence expansion 约 11ms。普通构建 source 查询为 168ms/140ms。
- **当前处理：** 生产默认预算不变。race-only 注入边界测试通过私有测试选项使用 5s，覆盖实际候选到模型 request 的 hostile-memory 数据边界；单独的 race 默认预算测试确认超时采取无候选的安全降级。
- **剩余风险：** race 下冷搜索正向路径在产品默认 750ms 下仍未通过。不得把 test-only 预算结果表述为默认预算召回成功；后续需决定是否优化 Mentle 搜索延迟或重新评估产品预算并跑完整矩阵。

## S09 原来源重放测试边界

- 重启后的 App 在独立子进程中；父测试进程不可直接持有该 ObserverHost。夹具现通过 remote control handler 在被测子进程内请求 ObserverHost 重投，不直接修改产品外部持久数据。
- 重新启用 trigger policy 后得到 `eligible` 并非重放导致：S08 新会话 B 在策略关闭时留下了一条尚未处理的新用户输入。最终 S09 用例保持策略关闭，单独断言 replay 未改写 canonical/tombstone、没有额外模型请求，并用新会话验证已删除事实不返回。

## 未解决验收项

- S08 V19：WORLD/ACTMEM 自动上下文隔离和显式工具读取审计。
- S09：除本次纠正/删除/重放因果链外的完整生命周期矩阵，以及其正式前置 gate。
- S11：backend disconnect/read-only/disk-full/index fault/recovery、workspace A/B 授权及产品固定 `diva` identity 的 workspace 语义。
- S01–S03：正式构建、真实后台与证据 schema/最终同候选前置 gate；这些未完成项阻止 Story Done。
