# v0.9.9 Laputa Clippy cleanup

修复 Rust 1.98 Clippy 在 `agent-diva-laputa` 中发现的 4 处
`unnecessary_sort_by`：ACTMEM 胶囊、Recall 反馈、Persona 请求和 Persona 历史均改用
`sort_by_key`，倒序排序使用 `std::cmp::Reverse`。

排序方向和业务行为保持不变。
