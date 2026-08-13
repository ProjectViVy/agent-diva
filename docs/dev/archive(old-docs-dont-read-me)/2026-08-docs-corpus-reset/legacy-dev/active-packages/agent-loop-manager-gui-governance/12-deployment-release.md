# 部署与发布

## 1. 发布模型

每个 G0–G5 子切片都是内部兼容重构，可按当前 `0.5.x` 线独立发布。不得积累到一次“大治理发布”才做真实 smoke。

## 2. 每个切片的发布门禁

- 相关 crate tests；
- Manager route/DTO contract；
- GUI Vitest/build（涉及 GUI 时）；
- Tauri debug/external gateway smoke；
- Tauri release/embedded gateway smoke（涉及 Host/runtime 时）；
- `just fmt-check && just check && just test`；
- iteration log 四件套；
- 明确 rollback commit。

## 3. 灰度

本计划不使用 runtime feature flag 双轨。灰度单位是“已经行为等价的内部切片”：

- 先合并纯提取/characterization；
- 再合并单 domain 委托；
- 每次发布后观察 error rate、startup、chat/tool/stop、session reload；
- 出现回归直接 revert 当期切片。

## 4. 配置和数据

默认无配置、schema、环境变量和数据迁移。发布说明必须明确写 `no migration required`。一旦某 slice 需要 schema 变化，必须从本计划拆出。

## 5. 完成发布

最终完成并不要求切换 major version，而要求：

- 旧巨型实现/重复 DTO/死 command 已删除；
- 所有 capability 状态真实；
- 全量 CI 与产品 smoke 通过；
- 架构文档与实际依赖一致；
- 性能无不可接受回归。
