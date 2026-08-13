# Verification

## Commands
- `cd agent-diva-gui && npm run build`
- `cargo check -p agent-diva-manager -p agent-diva-gui`

## Result
- 通过。`vue-tsc --noEmit` 与 `vite build` 均成功完成。
- 通过。`agent-diva-manager` 与 `agent-diva-gui`（Tauri）Rust 编译检查成功。

## Notes
- 本次变更仅涉及 `agent-diva-gui` 前端交互与文案兜底，不涉及 Rust workspace 代码。
- 未执行 `just fmt-check`、`just check`、`just test`；原因是本次未修改 Rust crate，验证重点为 GUI 模板、TypeScript 与构建通过性。
- 未执行桌面运行态人工 smoke；建议后续在 Tauri GUI 中补一轮人工验证，重点检查：
  - 再次点击已选供应商可取消选中。
  - 再次点击当前已激活模型后，保存链路不会再把空模型 fallback 回默认模型。
  - 选择模型后可通过“删除当前模型”清空当前主聊天配置。
  - 删除后该模型不会自动重新出现在快捷列表中。
  - 聊天页面模型下拉中每个已保存模型右侧均有独立删除按钮，删除当前正在使用项时会同步清空当前配置。
  - 供应商页顶部“刷新”按钮仅刷新供应商列表与状态，不会重新强制激活某个模型。
