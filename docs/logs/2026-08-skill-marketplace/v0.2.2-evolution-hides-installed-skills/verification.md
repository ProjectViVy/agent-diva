# v0.2.2 验证记录

## 命令与结果(2026-08-21)

| 验证项 | 命令 | 结果 |
|---|---|---|
| EvolutionView 测试 | `pnpm vitest run src/components/EvolutionView.test.ts` | 6/6 通过(含新增 `hides installed skills that are not evolution managed`) |
| GUI 全量测试 | `pnpm vitest run`(agent-diva-gui) | 68 文件 480/480 通过 |
| GUI 类型检查 | `pnpm exec vue-tsc --noEmit` | 通过 |

## 跳过说明

本迭代无 Rust 改动,`just fmt-check` / `just check` / cargo 测试不受影响,跳过
(上一提交 `17409ce7` 已通过全部门禁)。

## 延后项

- 桌面真机冒烟:Evolution → Skills 标签仅显示 Evolution 托管技能;已安装页仍可删除
  安装类技能。并入 `TODOLIST.md` 的 `SKILL-MARKETPLACE-DESKTOP-SMOKE`。
