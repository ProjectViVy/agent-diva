# Email C5-I Gate 2 implementation evidence

实现文件：`agent-diva-channels/src/adapters/email.rs`，worker commit `d385aec4`。
IMAP/SMTP 的 blocking 调用隔离在 `spawn_blocking`；adapter 保留 DIVA consent、TLS、auto-reply
和 multipart 语义，并借用 Octos 的 Message-ID/In-Reply-To/References 线程规则。

## 已接入行为

- `UNSEEN` 拉取、Message-ID/UID dedup、thread root 优先级、self-reply 与 allowlist 检查在
  admission 前完成；mark-seen 只有在 Fabric 接受后执行。
- RFC822 multipart 附件写入内容寻址 AttachmentStore；SMTP 回复返回稳定的 Message-ID 形状，
  失败保留原始错误上下文，不把发送失败当作成功。
- Email 不声明 Markdown、chunking、typing、edit/delete/reaction/card；unsupported 命令无
  IMAP/SMTP 副作用，取消会中断 blocking worker 等待。

证据位于 `tests/fixtures/c5/email/`：`plain.eml`、`multipart.eml`、`reply.eml` 及 README；
解析、线程、consent/self-reply、multipart、dedup/mark-seen 和 unsupported 测试已通过。
EM-01～EM-04 仍需 C5-V fake IMAP/SMTP transcript、busy/cancel 和健康探针证据。
