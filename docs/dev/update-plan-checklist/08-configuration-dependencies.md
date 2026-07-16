# 配置与依赖管理

## 配置

继续使用 `tools.builtin.update_plan` 布尔开关；默认值和迁移逻辑不变。普通 Agent 模式可注册，Plan mode 与 execution 模式不注册。

## 依赖

本修复不新增 Cargo、npm、系统服务或环境变量依赖。Rust 使用现有 serde，GUI 使用现有 Vue/Vitest。

## 数据与密钥

`update_plan` 不访问网络、不持久化清单、不读取或写入凭据。仓库 `TODOLIST.md` 仍由文件工作流和项目规则管理，不由工具自动修改。
