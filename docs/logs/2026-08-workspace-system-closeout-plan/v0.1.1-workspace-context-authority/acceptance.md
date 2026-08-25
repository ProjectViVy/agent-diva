# Acceptance

- Gateway 启动时 workspace source 由 CLI `WorkspaceContext` 解析结果携带到 Manager。
- `/api/workspace` 返回的 root/source 与运行时 context 一致，不依赖路径名称或 legacy home
  heuristic。
- GUI Tauri embedded runtime 和 CLI gateway 的构造入口均已适配完整 context。
- WS-00 完成后，WS-01 可以直接以 `/api/workspace` 作为唯一只读数据源接入 GUI。
