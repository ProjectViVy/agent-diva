# 验证记录

| 门 | 命令 | 结果 |
| --- | --- | --- |
| P0-A | `cargo test -p agent-diva-agent --lib world_shaped_content_requires_kind_domain_and_title` | PASS |
| P0-A | `cargo test -p agent-diva-agent --test world_claim_routing` | PASS |
| P0-B | `cargo test -p agent-diva-agent --lib prompt_guides_memory_tool_usage` | PASS |
| P1-C | `cargo test -p agent-diva-agent --lib helper_constructors_do_not_touch_machine_config_dir` | PASS |
| P1-C | `cargo test -p agent-diva-agent --lib frozen_core_is_captured` | PASS |
| P1-C | `cargo test -p agent-diva-agent --test compaction_integration context_injects_one_checkpoint` | PASS |
| P2-D | `cargo test -p agent-diva-core --lib crud` | PASS |
| P2-D | `cargo test -p agent-diva-cli --lib parses_command_and_plan` | PASS |
| P2-D | `cargo test -p agent-diva-cli --lib command_and_plan_decisions` | PASS |
| P2-D | `cargo test -p agent-diva-cli --lib default_headless_cancels_high_risk_plan_pending` | PASS |
| P2-D | GUI `npm test -- ApprovalCenter capabilities` | PASS（10 tests） |
| Clippy 跟随修复 | `just check` 初跑失败：`clippy::manual_strip` 于 `consolidation.rs` | 已修于 `accbcd97` |
| 收尾格式 | `just fmt-check` | PASS |
| 收尾 lint | `just check`（clippy -D warnings） | PASS |

未跑桌面 smoke。未跑全量 `just test`；分片测试覆盖 WORLD 丢弃、Prompt、构造器隔离、Approval/CLI/capability。
