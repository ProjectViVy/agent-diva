# Verification — GA-MEM-PARITY Wave 3（读侧闭环）

## 方法

每切片独立：`cargo fmt -p <crate>`、`cargo clippy -p <crate> --all-targets -- -D
warnings`、`cargo test -p <crate>`；S2 后相关 workspace 全量回归（CLI/GUI
按既有排除策略）。

## 结果

| 切片 | 提交 | 验证 |
|------|------|------|
| S1 | `3ae7975d` | laputa lib 35 + 集成 9 全绿；clippy 干净 |
| S2 | `e66cfa9b` | agent lib 386 + 集成 15 全绿；clippy 干净 |
| S3 | 本次 | docs only，`git diff --check` |

全 workspace 相关 crate 回归（除 CLI/GUI）：core 692 / laputa 35 / agent 386 /
tools 109 / manager 全绿。CLI 6 个既有 wiremock 502 失败
（`CLI-WIREMOCK-502-PREEXISTING`，TODOLIST 已记录，与本迭代无关）。

## 验收测试落点（D2/D3/D4 + F5/H3 + U1/U2 自动化）

- D4：`recall_returns_prompt_block_with_recalled_content`（prefetch prompt_block
  实际含召回关键字，证 FTS → prompt_block 通路）。
- D2/D3：`legacy_prefetch_failure_leaves_messages_untouched`（turn messages
  保持 [system, user]，Legacy Failed 不扩散）；manager.rs:292-307 明确 reason
  文案可读。
- F5/H3：
  - `memory_add_visible_in_next_startup_rendering`（新 provider 实例 startup
    markdown 含记录 id 与预览）。
  - `apply_then_search_remains_consistent_across_reopens`（新 provider 实例
    search 命中）。
  - `checkpoint_invisible_to_search_and_subsequent_startup`（session end 后
    新 provider 实例 search 空 + startup 不含 checkpoint）。
- G3/U3（读侧投影）：
  - `supersedes_tombstone_filtered_from_next_startup_rendering`（startup 过滤
    supersedes 目标）。
  - `supersedes_tombstone_filtered_from_search`（search_visible 过滤 supersedes
    目标）。
- B2：`startup_respects_l1_budget_ceiling`（预算 2 行只渲染 2 条）。
- G3 proposal-first：`memory_remove_creates_governed_proposal`（typed
  memory_remove 返回 ProposalCreated）。
- 注入顺序（D4 端到端）：`typed_prefetch_inserts_after_working_memory_block`
  （[system, working_memory, prefetch, user]）。
- D2/D4 空路径：`no_injection_when_both_prefetch_and_working_memory_are_absent`。

## 真实 bug 修复（测试暴露）

supersedes tombstone 此前只过滤「自身是 tombstone 的记录」，不处理「被
tombstone 指向的目标记录」——导致用户 `memory_remove` 流程下 proposal apply
后，目标记录在下次 startup 与 search_visible 中仍可见。修复：新增
`TypedMemoryStore::superseded_target_ids()`（JOIN memory_supersedes 与
memory_records WHERE tombstone=1）并在 startup L1 渲染与 memory_search 两处
接线过滤。修复后两个 wave3 测试通过；既有 27 个 laputa 测试与 383 个 agent
测试无回归。

## 遗留

- **agent-diva-files clippy 失败（既有，与本迭代无关）**：
  `src/s3.rs:247` 的 `empty_line_after_doc_comments` 警告在 Wave 2 与 Wave 3
  baseline 上均可复现（stash 验证）。Wave 3 未触及该 crate，按「不修与本
  Wave 无关的预存在问题」原则保留。归后续清理 Wave 或独立 TODO。
- F4 同会话热注入：apply 后同会话刷新 startup cache 或触发 re-prefetch 机制
  未实现（startup_markdown 在 open() 一次性渲染缓存；typed_provider.rs:87）。
  归 Wave 5 或独立 slice（TODOLIST `F4 同会话热注入` 条目）。
- F3 GUI/CLI 审批 memory 域端到端验收：归 GMH-52 / G2D+ 桌面验收。
- F6 Rollback 端到端验收：`rollback_governed` API 已具备，缺 apply→rollback
  →FTS 清退→startup 不再出现的端到端测试。归 Wave 5。
- F7 tombstone U3 完整路径：Wave 3 仅证读侧投影过滤；proposal→apply→tombstone
  →下次会话不再出现→GUI 可见历史的完整路径归 Wave 5 GC 或 GMH-52。
- U1/U2 真机联通 smoke：自动化证据已具备；真机 CLI/GUI 联通归 G2D+ 桌面验收
  （按规则 `smoke-test-required-for-user-visible-change`）。
- CLI wiremock 502 排查（独立 TODO，`CLI-WIREMOCK-502-PREEXISTING`）。
