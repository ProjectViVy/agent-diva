# Release

本修复以本地 `dev` 分支上的 focused commit 交付，不推送、不生成安装包。

开发者需退出当前 Tauri 应用并重新运行 `pnpm tauri dev`、`just start` 或
`just make-diva`；只重启旧的外部 Gateway 不会加载新的生命周期策略。
