# Acceptance — GA-MEM-PARITY Wave 5

## 自动化验收（已通过）

### F6：Rollback 端到端
- governed apply → rollback → FTS 不再命中 + startup 渲染不含目标 ✓
- rollback 幂等（第二次返回 false）✓

### F7：Tombstone 生命周期
- tombstone → 下次 startup + search 过滤 ✓
- 不存在 id 的 tombstone 不崩溃 ✓

### Working memory GC
- `gc_session_scoped` 只清目标 session ✓
- `gc_stale_session_scoped` 清孤儿记录 ✓
- 空库 GC 返回 0 不报错 ✓

### Superseded gate
- superseded digest 阻止候选 ✓
- superseded 优先级高于 duplicate ✓
- 新内容不受 superseded 影响 ✓

### Consolidation 条目化
- distill 已运行时 consolidation 跳过 ✓
- items 数组逐条 dispatch 到 memory_add ✓
- 非数组降级到 sync_turn ✓

## 延期验收（归 G2D+ / 独立 Wave）

| 项 | 原因 |
|----|------|
| F4 同会话热注入 | 需要 startup cache invalidate 机制，本 Wave 不吞下 |
| F7 GUI 可见 tombstone 历史 | 需要 Tauri GUI 联通，归 G2D+ 桌面验收 |
| B9 tool-result 强制校验 | 改动面大，归后续独立 Wave |
| `memory_list` superseded 过滤 | S3 发现的读侧缺口，记录为延期项 |
| G1–G12 真机端到端 | 均为产品/真机验收，归 G2D+ |
