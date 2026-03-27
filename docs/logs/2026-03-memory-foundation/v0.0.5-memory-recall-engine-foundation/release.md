# Release

## Scope

本轮是 memory foundation 的内部演进，不涉及新的部署入口、配置项或外部发布动作。

## Release Method

- 无需单独发布流程。
- 改动随常规 workspace 构建产物生效。
- 对外行为保持兼容：
  - `agent-diva-core::memory` 无 breaking change
  - `WorkspaceMemoryService` 公开接口保持稳定
  - memory tools schema 保持不变

## Rollout Risk

- 主要风险在 recall 排序和 `MEMORY.md` chunking 质量，而不是接口破坏。
- 本轮已用 crate 级测试和全量测试覆盖基本回归面。
