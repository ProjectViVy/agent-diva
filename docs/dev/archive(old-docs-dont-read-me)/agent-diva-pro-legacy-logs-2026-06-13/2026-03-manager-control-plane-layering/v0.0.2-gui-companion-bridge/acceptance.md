# Acceptance

- 在 GUI 中打开 provider 设置页，列表仍能正常展示 provider 基础信息和模型列表。
- 在 GUI 中创建、删除自定义 provider，以及新增/删除 provider model，行为保持可用，且实际调用走 manager companion API。
- 在 GUI 中获取 runtime config、config status、gateway process status、service status 等本地宿主管理能力时，行为保持不变。
- `agent-diva-cli` 的 `gateway run` 在 `full` 与 `nano` feature 下仍可分别编译通过，`--remote` 语义保持不变。
