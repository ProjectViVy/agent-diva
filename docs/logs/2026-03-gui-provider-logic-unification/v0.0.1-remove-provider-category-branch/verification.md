# Verification

## Workspace Validation

1. `just fmt-check`
   - 结果：通过。

2. `just check`
   - 结果：通过。

3. `just test`
   - 结果：未完成。
   - 原因：Windows 环境中已有进程占用 `target\debug\agent-diva.exe`，`cargo test --all` 在重建该二进制时返回 `os error 5`（拒绝访问），不是测试断言失败。

## Targeted Tests

1. `cargo test -p agent-diva-providers`
   - 结果：通过。
   - 关键覆盖：
     - `litellm::tests::test_named_provider_non_native_base_adds_litellm_prefix`
     - `litellm::tests::test_named_provider_native_base_keeps_raw_model`
     - `litellm::tests::test_direct_provider_base_keeps_raw_model`

2. `cargo test -p agent-diva-gui`
   - 结果：通过。

3. `cargo test -p agent-diva-manager`
   - 结果：通过。
   - 关键覆盖：
     - `manager::tests::normalize_model_for_provider_keeps_explicit_model_for_explicit_provider`
     - `manager::tests::normalize_model_for_provider_replaces_cross_provider_model`

## Smoke

1. `.\target\debug\agent-diva.exe --help`
   - 结果：通过。
   - 说明：由于 `cargo run` 需要重写被占用的 `target\debug\agent-diva.exe`，故改为直接执行现有已编译产物完成最小 CLI smoke。
