# Release — GA-MEM-PARITY Wave 3（读侧闭环）

## 发布方式

代码变更随常规 crate 构建发布（laputa / agent），无独立部署物。

## 行为变更提示

- **启动渲染与 `memory_search` 新增 supersedes-target 过滤**：被 supersedes
  tombstone 指向的目标记录不再出现在 L1 索引与 search 结果中。修复此前真实
  bug——用户 `memory_remove` 走完 governed apply 后，目标记录在下次 startup
  与 search_visible 中仍可见的问题。
- **`TypedMemoryStore` 新增公开方法**：`superseded_target_ids()` 返回当前
  store 中所有 supersedes tombstone 的 target id 集合（HashSet<String>），
  供读侧投影复用。

## GUI/CLI 影响

- 无 wire/SSE/Tauri 协议变更。
- 行为变更仅影响"被 tombstone 指向的记录"的可见性（更严谨的过滤）；对无
  supersedes tombstone 的 workspace 零影响。
- 测试新增 11 个（laputa +8 / agent +3），无 CLI 行为变更。
