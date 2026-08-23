# Release: pet → mate 全面改名

## 发布方式

本迭代为纯改名 + 兼容迁移，不改变运行时行为，无需独立发布流程：

- 随下一个常规版本发布（当前工作区版本 `0.9.9`，未单独 bump）。
- GUI 用户重新构建/安装后即可生效；Windows 打包沿用
  `scripts/package-windows-gui.ps1` 常规流程。

## 用户侧影响

- 旧 `~/.agent-diva/config.json` 中的 `"pet"` 配置节自动迁移为 `"mate"`，
  用户设置不丢失；下次保存时文件键名更新为 `"mate"`。
- GUI localStorage 旧键一次性迁移，伙伴（原桌宠）的语音/模型/外观设置保留。
- VRM 模型与语音资源目录（`vrm/`、`voice_resource/`）路径未变，
  已导入资源无需重新导入。

## 回滚

如需回滚，revert 本迭代提交即可：serde `alias = "pet"` 与 localStorage
迁移均为单向兼容设计，回滚后旧逻辑读取 `"pet"` 键时，对已写出 `"mate"`
键的配置文件将无法读到该节（回退默认值），属于可接受的回滚代价；
localStorage 旧键在迁移后仍保留未删除，前端回滚可继续读取旧键。
