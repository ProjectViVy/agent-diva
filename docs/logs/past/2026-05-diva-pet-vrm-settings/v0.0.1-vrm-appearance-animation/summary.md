# Diva 桌宠 VRM 外观与动画修复总结

## 变更内容

- 将桌宠设置面板调整为“外观设置”，仅保留“外观 / 动画”两个页签。
- 外观页直接管理当前 VRM 模型，支持内置模型与自定义模型列表。
- 自定义 VRM 模型导入到用户配置目录 `~/.agent-diva/vrm/models/custom/`。
- 新增 Tauri 命令：列出、导入、删除、读取自定义 VRM 模型。
- 外观切换同步应用模型、外观 ID、待机动作集合、动画开关、表情开关。
- 动画页改为管理真实 runtime motion catalog 对应动作，支持待机开关、待机动作选择、one-shot 预览与停止。
- 嵌入式桌宠和桌面宠运行时均接入 motion state。

## 影响范围

- `agent-diva-gui` 前端桌宠设置、嵌入式桌宠、桌面宠叠加层。
- `avatar-runtime-vrm` motion controller 状态。
- `shared-avatar-protocol` motion state 协议。
- `agent-diva-gui/src-tauri` VRM 用户资源命令。
