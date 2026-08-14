# I1-S4 验收步骤

## 自动化验收

1. 运行 `just fmt-check`、`just check`、`just test`。
2. 在 `agent-diva-gui` 运行 `pnpm test` 与 `pnpm build`。
3. 运行 `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml`。

## 真实桌面验收（待执行）

1. 启动 Manager 与 Tauri，确认 Evolution 只有 Skill 与待审两类主入口。
2. 编辑已有 Skill；用旧 hash 模拟冲突，确认草稿与选择保留；再成功保存并加载历史。
3. 停用 Home 覆盖，确认不回落内置；硬删 Home 后确认同名内置重新显示。
4. 从 Settings 分别用本地 ZIP 与 Marketplace 新装 Skill；确认重复 slug 被拒绝且不能覆盖。
5. 创建带 attestation 的用户请求；触发 `memory_distill` 与 AutoDream 请求；确认未接受前
   不产生 Skill 文件，stale 请求不可接受。
6. 接受请求并创建新 Session，确认索引与 `skill_read` 能发现；子代理与 cron 不具备
   `skill_read`/`memory_distill` 写入能力。
7. 打开 Chat AutoDream 卡、Notebook 与 Settings，确认不调用旧 Laputa proposal 路由。

未完成第 2–7 步前，不勾选 `UI-S4-EVOLUTION-SKILL`。
