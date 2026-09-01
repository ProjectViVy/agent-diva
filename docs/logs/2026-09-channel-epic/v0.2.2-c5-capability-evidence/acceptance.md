# Acceptance

## 离线验收

1. 检查 `evidence-manifest.md` 与六份 `platforms/*-gate3.md`，确认每条 `verified` 行
   都可追溯到 fixture、Rust 测试、精确请求/响应和 receipt/error。
2. 运行：

   ```text
   cargo test -p agent-diva-channels --all-targets
   cargo clippy -p agent-diva-channels --all-targets -- -D warnings
   just msrv-probe check -p agent-diva-channels
   just fmt-check
   just check
   just test
   git diff --check
   ```

3. 确认 unsupported capability 在首次平台调用前返回 typed
   `UnsupportedCapability`，且日志不包含 secret、token、signed URL、完整 open ID、
   原始媒体或审批 payload。

## QQ 纵向验收

在仓库外注入四个 QQ 环境变量后，显式运行：

```text
cargo test -p agent-diva-channels --test qq_live_harness -- --ignored --nocapture
```

应按固定顺序观察 C2C、群 @、重放去重、冻结地址的 reply ID/真实 response ID、single
final、`Accepted` receipt、人工断线后的 resume/reconnect，以及 stop 对 listener、
heartbeat、token wait、backoff 的共同终止。缺少凭据或权限时只记录阻断原因，不能把测试
成功返回当作 live smoke 完成。

## 阶段结论

在 QQ live smoke、D-013/D-014 官方证据和全部 workspace 门禁均通过前，C5-V/C5-Q 保持
open，禁止进入 C6 Manager 切换。
