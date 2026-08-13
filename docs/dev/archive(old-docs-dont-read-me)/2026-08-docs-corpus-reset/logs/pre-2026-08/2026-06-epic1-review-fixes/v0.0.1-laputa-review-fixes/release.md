# Epic 1 评审修复发布记录

## 发布方式

本轮未执行单独部署。交付物是一个聚焦源码提交，可纳入下一次正常 workspace build 或 release train。

## 回滚

如果发现回归，回滚本轮聚焦提交即可。变更范围限定在 Laputa authority storage/apply/rollback/event 行为、manager Laputa SSE 处理、Tauri Laputa 命令错误形态、session atomic save 耐久性，以及 BMAD/story 跟踪文档。

## 运维备注

预存 sandbox 编译阻断修复前，不要把 GUI/Tauri compile check 作为本轮 release gate。
