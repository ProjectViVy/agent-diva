# v0.2.1 验证记录

## 命令与结果(2026-08-21)

| 验证项 | 命令 | 结果 |
|---|---|---|
| core skill_home 单测 | `cargo test -p agent-diva-core skill_home` | 7/7 通过(含新增 `evolution_managed_reflects_accepted_proposals`) |
| core+manager 编译 | `cargo build -p agent-diva-manager -p agent-diva-core` | 通过 |
| 新组件测试 | `pnpm vitest run src/components/settings/InstalledSkillsTab.test.ts` | 5/5 通过 |
| GUI 全量测试 | `pnpm vitest run`(agent-diva-gui) | 68 文件 479/479 通过 |
| GUI 类型检查 | `pnpm exec vue-tsc --noEmit` | 无输出(通过) |
| 格式化 | `cargo fmt --all` + `just fmt-check` | 通过 |
| clippy 全工作区 | `just check`(含 agent-diva-gui src-tauri) | 通过(-D warnings) |
| 工作区测试 | `cargo test --workspace --exclude agent-diva-cli --exclude agent-diva-gui` | 58 套件全绿 |

## 排除说明

`just test` 会因用户正在运行的 `agent-diva.exe` / `agent-diva-gui.exe` 锁定
`target\debug` 二进制而失败(os error 5),故按既往迭代惯例排除这两个 crate 打包产物,
其源码已通过 `just check` 覆盖。

## 延后项

- 桌面真机冒烟(重建网关 + GUI 后按 `acceptance.md` 执行),记入 `TODOLIST.md`
  `SKILL-MARKETPLACE-DESKTOP-SMOKE`。
