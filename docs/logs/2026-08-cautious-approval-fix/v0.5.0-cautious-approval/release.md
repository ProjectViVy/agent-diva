# Release / 发布

## 发布方式

- 本次为 CLI + GUI 联修，需要同时发版：
  - `agent-diva-cli` 二进制（`cargo build --release -p agent-diva-cli`）；
  - `agent-diva-gui` Tauri 安装包（`pnpm tauri build`）。
- 建议在下一个 minor 或 patch 版本（例如 `v0.5.x`）合并发版。

## 发布前检查

- [ ] `just ci` 全绿；
- [ ] `acceptance.md` 五项 GUI 验收场景人工通过；
- [ ] `CHANGELOG.md` 记录：
  - `fix(sandbox): escalate ExecutionFailed to approval so cautious mode prompts`;
  - `fix(gui): wire permissionMode to backend approval policy`;
  - `feat(gui): auto-open Approval Center drawer on new pending request`.

## 兼容性

- `ChatRequest.approval_policy` 为 `Option`，旧 GUI 不传即为 `None`，行为等同修复前；
- `RuntimeControlCommand::SetApprovalPolicy` 新增变体，外部若裸 `match` 该 enum 需补分支（仅影响 manager 内部）。
