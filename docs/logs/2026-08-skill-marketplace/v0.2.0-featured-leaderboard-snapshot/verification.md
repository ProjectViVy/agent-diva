# v0.2.0 Featured Leaderboard Snapshot — Verification

## 脚本

- `python agent-diva-manager/scripts/fetch_marketplace_featured.py`
  无 token 实跑成功：`wrote 100 featured skills (web:leaderboard)`；
  YAML 首条 `vercel-labs/skills/find-skills`（846,620 周安装量和），
  与 skills.sh 首页榜单一致。
- v1 API token 路径未验证（无 token）；逻辑为直接分页 GET + Bearer，
  已记入 TODOLIST `SKILL-MARKETPLACE-V1-TOKEN-VERIFY`。

## manager

- `cargo test -p agent-diva-manager --lib marketplace`：7 passed
  （含新增 `featured_snapshot_parses_embedded_yaml`）。
- `cargo test -p agent-diva-manager --test skill_marketplace_e2e`：2 passed
  （含新增 `marketplace_featured_serves_embedded_snapshot`：200、status=ok、
  total 与数组一致、generated_at 非空、全部 id 为 owner/repo/slug）。

## GUI

- `npx vitest run`：67 files / 474 tests 全部通过（新增 2 条 featured 测试：
  空态排序 + 快照标题、精选安装传完整 id）。
- `npm run build`（vue-tsc + vite）：构建成功。
- `cargo check -p agent-diva-gui`（src-tauri）：通过（仅上游 imap-proto
  future-incompat 警告，预存在）。

## 工作区门禁

- `just fmt-check`：通过。
- `just check`（clippy -D）：通过。
- `cargo test --workspace --exclude agent-diva-cli --exclude agent-diva-gui`：
  exit 0，58 个 suite 全绿。排除原因：用户正在运行的 agent-diva.exe /
  agent-diva-gui.exe 锁住 debug 二进制（`failed to remove ... 拒绝访问`），
  与上一迭代相同的预存在环境约束，重启后复跑即可。

## 未验证项

- 真机桌面冒烟（重建 + 重启网关/GUI）：见 acceptance.md 与 TODOLIST
  `SKILL-MARKETPLACE-DESKTOP-SMOKE`。
- v1 API token 路径：见 TODOLIST `SKILL-MARKETPLACE-V1-TOKEN-VERIFY`。
