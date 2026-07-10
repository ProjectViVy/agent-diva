# 验证

- `cargo fmt --all -- --check`：通过。
- `cargo test -p agent-diva-core planning::store::tests::test_delete_expired_plans_removes_all_statuses_and_cascades`：通过。
- `cargo test -p agent-diva-manager planning_service::tests::list_plans_cleans_up_expired_plans`：通过。

定向测试覆盖过期/未过期规划、所有规划状态、级联删除子表及 `active_plan` 关联。测试编译期间报告了 `agent-diva-core/src/supervised/store.rs` 中既有的未使用变量警告，不属于本次改动。
