# Acceptance

产品/架构验收步骤：

1. 在隔离分支运行 `cargo test -p agent-diva-channels --all-targets`，确认六个 native 模块、
   runtime/shared TCK 和既有 QQ reconnect integration 均通过。
2. 查看 `build_active_adapters`：启用 Telegram/Discord/Feishu/DingTalk/Email/QQ 时返回六个
   concrete adapter；默认配置为空集合；该函数不启动 listener、不触碰 Manager 注册。
3. 对每个 adapter 检查 capability snapshot 与 `evidence-manifest.md`：声明为 false 的
   command 必须在第一笔 HTTP/WS/IMAP/SMTP 调用前返回 `UnsupportedCapability`。
4. 检查 `tests/fixtures/c5/` 和 `platforms/*-gate2.md` 的 endpoint、身份、receipt、失败
   与限制说明；不把 fixture 形状当作外部真实发送成功。
5. QQ 额外确认 D-013/D-014 仍为 blocked/unsupported，凭据只能在仓库外注入；完成 C5-V 前
   不得将 C5 或 C6 标记为完成。
