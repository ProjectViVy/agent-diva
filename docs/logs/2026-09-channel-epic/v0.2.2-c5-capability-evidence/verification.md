# Verification

## 已完成的定向验证

以下均在 `C:\Users\Administrator\Desktop\morediva\agent-diva-channel-epic`、
`feat/channel-epic` 上执行：

- Telegram adapter tests：7 passed。
- Discord adapter wire tests：9 passed。
- Feishu adapter tests：10 passed。
- DingTalk adapter tests：10 passed。
- Email adapter tests：11 passed。
- QQ adapter tests：10 passed。
- `cargo test -p agent-diva-channels --all-targets`：通过；频道单元/运行时/TCK/
  characterization/reconnect 测试分别为 142、11、9、5、6 passed，QQ live harness 的 1 个
  test 按设计 ignored。
- `cargo test -p agent-diva-channels --test channel_adapter_shared_tck`：9 passed。
- `cargo clippy -p agent-diva-channels --all-targets -- -D warnings`：passed。
- `just msrv-probe check -p agent-diva-channels`：passed；仅调整 `Cargo.lock` 后在 Rust
  1.80.1 上完成 channel package probe，未提高 MSRV 或修改 manifest dependency constraint。
- `just fmt-check`：passed。
- `just check`：passed。
- `just test`：passed，完整 workspace tests/doc-tests 退出码为 0。
- `cargo test -p agent-diva-channels --test qq_live_harness`：harness 编译成功，1 test
  为 `ignored`；未提供凭据时不连接外部平台。
- `git diff --check`：最终文档更新后再次执行并通过。

共享 TCK 额外确认：六频道静态 capability snapshot 与冻结矩阵相等、每频道至少一个
fixture 可解析、代表性 unsupported command 在 transport spy 前返回
`UnsupportedCapability`、evidence manifest 恰有 29 个唯一 ID，且 `verified` 行具有
非空 fixture/test/request/response 证据。

## 最终门禁记录

最终门禁记录如下；任何失败都不允许将 C5 标记为完成：

```text
cargo test -p agent-diva-channels --all-targets       [passed]
cargo clippy -p agent-diva-channels --all-targets -- -D warnings [passed]
just msrv-probe check -p agent-diva-channels          [passed after Cargo.lock pins]
just fmt-check                                        [passed]
just check                                            [passed]
just test                                             [passed, exit code 0]
git diff --check                                      [passed]
```

第一次完整 `just test` 因本 worktree 的可重建 `target\\debug` 产物耗尽磁盘空间而失败；清理
该精确生成目录后重跑，第二次完整结果为通过。已知工作区级风险仍单独记录在
`TODOLIST.md`：Manager Neuro-Link loopback 偶发竞态、尚未声明完成的全 workspace MSRV
审计，以及其它非频道测试 flake；它们不能被频道定向测试结果掩盖。
