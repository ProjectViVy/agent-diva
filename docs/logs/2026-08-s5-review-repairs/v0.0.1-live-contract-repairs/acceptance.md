# 验收步骤

1. 巩固路径：带 `kind: world` + `domain` + `title` 的 `save_memory` 项不得调用 `memory_add`，且不得创建 `.laputa/cognitive`。
2. 系统 Prompt：出现 `memory_add` / `memory_remove`，不出现 `proposal_created` 或 “not effective until approved”。
3. 测试构造器：`ContextBuilder::new(temp)` / `with_skills(temp)` 不改真实 `{config_dir}/memory`。
4. Approval Center：领域筛选项只有 Command / Plan，无 Memory。
5. 生产主路径仍走 `ContextBuilder::with_skill_home` + `MemoryHome`。

桌面视觉验收仍挂 UI-S2/S3/S4 smoke，本切片不做。
