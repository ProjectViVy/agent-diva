# Verification

- `cargo test -p agent-diva-agent --lib test_image_attachment_is_forwarded_as_structured_multimodal_user_message -- --nocapture`
  - 结果：通过。验证图片附件会作为结构化多模态 user message 发给 provider。
- `cargo test -p agent-diva-agent --lib test_image_attachment_rejects_non_vision_model_before_provider_call -- --nocapture`
  - 结果：通过。验证非视觉模型会在 provider 调用前被明确拒绝。
- `cargo test -p agent-diva-agent --lib build_current_turn_message_combines_text_and_image_parts -- --nocapture`
  - 结果：通过。验证当前轮文本与图片 part 的组装逻辑。
- `cargo test -p agent-diva-agent --lib`
  - 结果：通过（320 tests passed）。

补充说明：

- 运行 `cargo test -p agent-diva-agent <test-name>` 时，cargo 还会尝试编译预先存在的 `agent-diva-agent/tests/compaction_real_test.rs`，该测试依赖已移除的 compaction API，导致 package 级 targeted test 命令失败。该问题与本次改动无关，已记录到 `TODOLIST.md`。
