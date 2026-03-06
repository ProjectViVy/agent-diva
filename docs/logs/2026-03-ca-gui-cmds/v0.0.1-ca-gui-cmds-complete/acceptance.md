# CA-GUI-CMDS 验收步骤

## 验收清单

- [x] `just fmt-check && just check && just test` 通过
- [x] `cd agent-diva-gui && pnpm tauri dev` 可启动，中控台与设置页无报错
- [x] Windows：服务管理按钮可触发 `agent-diva service *`，状态正确（打包应用）
- [x] Linux：服务管理可调用 systemd 脚本（需 pkexec）（打包应用）
- [x] macOS：`bundle:prepare` 后 `resources/launchd/` 存在；服务管理显示“待接入”提示（符合 WBS 受控降级）
- [x] 优先级.md 中 CA-GUI-CMDS 已标记【已完成】
- [x] 迭代日志已创建：`docs/logs/2026-03-ca-gui-cmds/v0.0.1-ca-gui-cmds-complete/` 含 summary.md、verification.md、acceptance.md

## 用户视角验收

1. 打开 Agent Diva GUI，进入中控台，可查看 gateway 进程状态、配置编辑器、日志面板
2. 点击“启动网关”/“停止网关”，进程可正常启停
3. 配置编辑器可加载、保存 JSON 配置
4. 日志面板可刷新并显示最近日志
5. 设置 → 通用：开发模式下显示“服务管理（仅打包应用可用）”；打包应用下按平台显示服务管理面板
