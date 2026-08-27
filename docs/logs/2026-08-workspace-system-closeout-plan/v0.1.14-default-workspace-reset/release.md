# Release

本功能以本地 `dev` 分支 focused commit 交付，不推送、不单独生成安装包。

重新加载 GUI 后可在“设置 → 工作区”保存或重置全局默认目录。该操作不重建当前 runtime 的
内嵌 Gateway；聊天入口切换到目标 workspace/session authority 时仍遵守原有运行态 guard。

自动化门禁与一次 Windows Tauri 真机启动烟测均通过；未执行 push。
