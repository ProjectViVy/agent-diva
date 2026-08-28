# Batched dirty-tree cleanup — v0.1.0

日期：2026-08-28

## 本迭代结果

- 按用户确认删除已不再使用的 `.vibeyardignore`，提交为 `9fd1ecfe`。
- 将 `LOCK.md` 的历史记录与对应归档文件成对收口，提交为 `f9ebb984`。
- 保留并提交 12 份已有内容的 crate-local `agents.md` 指南，提交为 `33320c9f`。
- 刷新 `agent-diva-gui/src-tauri/Cargo.toml` 的 Git 索引状态；文件内容与 `HEAD` 一致，未产生代码变更。

本迭代只处理既有 dirty 文件和文档归档，不修改产品运行时逻辑。
