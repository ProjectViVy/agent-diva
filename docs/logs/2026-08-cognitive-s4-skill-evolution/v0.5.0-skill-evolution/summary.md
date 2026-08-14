# I1-S4 Skill Evolution 总结

## 结果

I1-S4 已实现机器级 Skill 权威与 SkillProposal 人审链路。生产路径只读取
`{config_dir}/skills/` 与只读内置 Skill，不读取或迁移 `workspace/skills`。

## 变更

- `SkillHome` 统一处理严格 slug/frontmatter、Home 覆盖、禁用、CAS、no-op、历史、
  ZIP 新装、密钥拒绝、请求唯一性与孤儿头修复；ZIP 在创建目标目录前完成全包
  路径/密钥预检，写入失败会清理未完成目标，避免半安装状态。
- Agent 的稳定前缀只含启用 Skill 的 slug/单行描述；always 正文有单项与总预算；
  主代理提供只读 `skill_read` 与仅创建请求的 `memory_distill`，子代理/cron 无写能力。
- AutoDream 在整理 ACTMEM Work 后执行受限 Skill 反思，最多创建八条 `autodream`
  待审请求；provider/schema 失败降级为零请求，不回滚 Work。
- Manager 与 Tauri 提供同名 Skill/Evolution API、CAS 与结构化错误；ZIP/Marketplace
  仅允许创建不存在的 slug。
- Evolution GUI 重建为 Skill 与待审双入口；Chat 读取 Skill request，Notebook 删除
  SOP/Skill/Memory proposal 动作，Settings 仅保留安装入口。

## 提交

- `7e25a713` `feat: add machine-wide skill authority`
- `2e3112ca` `feat: route agent skills through machine authority`
- `8c4e0d3c` `feat: create skill requests from autodream`
- `9b643f81` `feat: expose skill evolution APIs`
- `de5dc35f` `feat: rebuild evolution around skills`
- `8ddb9776` `fix: preflight skill package installs`

S5 的旧物理符号删除与 S6 最终零残留证明未进入本阶段。
