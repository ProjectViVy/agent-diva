# Discord C5-V Gate 3 evidence

状态：`partial`。本页只审计 DC-01、DC-02、DC-03、DC-05；四行均保留
`partial`，没有足够的现有 fixture、测试源码和既有通过记录把任何一行升级为
`verified`。DC-04 的既有窄行证据保留在文末，本次未重新判定。

## 审计基线

- Octos 固定 checkout：`C:\Users\Administrator\Desktop\morediva\.workspace\octos`，
  `HEAD=5ea987813de4fd2afdd1d78f2106ad2868f0d923`，工作树无改动。
- Octos 对照文件：`crates/octos-bus/src/discord_channel.rs`、
  `crates/octos-bus/src/channel.rs`、`crates/octos-bus/src/dedup.rs`。
- DIVA 对照文件：`agent-diva-channels/src/adapters/discord.rs`；冻结矩阵来自
  `agent-diva-channels/tests/channel_adapter_shared_tck.rs:227-252`，DIVA 静态
  capability snapshot 位于 `agent-diva-channels/src/adapters/discord.rs:170-207`。
- 既有通过记录来自
  `docs/logs/2026-09-channel-epic/v0.2.2-c5-capability-evidence/verification.md`：
  Discord adapter tests 记录为 9 passed；本次审计没有重新编译、运行测试或访问网络。
- `tests/fixtures/c5/discord/gateway-frames.json:1-9` 只在共享 TCK
  `channel_adapter_shared_tck.rs:428-458` 中作 JSON 可解析性输入；Gateway wire
  测试在 `discord.rs:1701-1768` 内联构造 WS transcript。`rest-responses.json`
  没有被 adapter 或 Discord wire test `include_str!` 消费；REST 测试在
  `discord.rs:1364-1390` 内联构造响应。因此下面把这些文件区分为“fixture 形状存在”
  与“端到端请求断言存在”。

## Octos 稳定源码锚点与共享语义

| Octos symbol / stable anchor | 源码语义 | DIVA 对应 symbol / anchor |
| --- | --- | --- |
| `discord_channel.rs:75-150` `Handler::message` / `Handler::ready` | Serenity 回调先过滤 bot，再用 `MessageDedup` 和 `allowed_senders`，下载附件，构造 `InboundMessage`；`ready` 只记录连接日志 | `consume_gateway:328-459`、`parse_incoming_message:1074-1145`、`handle_incoming:461-570` |
| `discord_channel.rs:166-190` `DiscordChannel::start` | 固定 `GUILD_MESSAGES | DIRECT_MESSAGES | MESSAGE_CONTENT`，交给 Serenity `Client::start`；HELLO/IDENTIFY/READY/重连由库封装 | `discover_gateway:260-281`、`run_gateway:283-326`、`consume_gateway:328-459`、`start:942-957` |
| `discord_channel.rs:197-226` `send_with_id` | 数字 `channel_id`；有媒体时 `CreateAttachment::path` + `send_files`，并把 `msg.content` 放入同一 message builder；无媒体时 `say`；返回真实 snowflake | `send_message:573-687`、`send_attachments:689-729` |
| `discord_channel.rs:228-304` `edit_message` / `delete_message` / `react_to_message` / `remove_reaction` | Serenity 映射到消息编辑、删除、添加 reaction、删除当前 bot reaction | `execute_edit:776-803`、`execute_delete:805-835`、`execute_reaction:837-872` |
| `discord_channel.rs:306-337` `send_embed` | `CreateEmbed` 设置 title/description/color/fields 后发送并返回 ID | `send_message:605-610`、`discord_embed:1190-1205` |
| `channel.rs:43-69`, `:241-247` typing/listening/health defaults | 默认 typing/listening 是 no-op/fallback，默认 health 是 `Unknown`；Octos Discord 未覆写这两个接口 | `execute_typing:874-903`、`probe_health:905-928` |
| `channel.rs:78-83`, `:148-151`, `:202-215` | `send_with_id` 默认返回 `None`，edit/delete/reaction 默认成功 no-op；Discord 专用实现才产生 REST 副作用 | DIVA `ChannelAdapter::execute:959-1019` 先 `ensure_supported` 再调用具体 REST |
| `dedup.rs:19-74` `MessageDedup` | LRU capacity 1000、TTL 60s；空 ID 永不去重；检查时记录，失败处理可 `forget` | DIVA 使用 `GatewayState` + `seen: HashSet<String>` (`discord.rs:62-68`, `:122-131`)；没有 TTL/LRU/forget/原子 reserve |

## 分配行审计结论

### DC-01 — Gateway opcode / heartbeat / resume

- 冻结矩阵目标：`ReliabilityHeartbeat`、`ReliabilitySupervisedRestart`
  （`channel_adapter_shared_tck.rs:249-252`）。矩阵没有 `ReliabilityResume`；当前
  JSON 也把 `ReliabilityResume` 列在 Discord `blocked_unsupported`，所以不能仅凭
  DIVA 中存在 op 6 分支就宣称 resume 已验证。
- Octos 对照：`crates/octos-bus/src/discord_channel.rs:75-150`
  （`Handler::message`、`Handler::ready`）和 `:166-190` (`start`) 把 Gateway
  状态机交给 Serenity；Octos 源码没有可审计的独立 op 6/7/9 transcript、heartbeat
  ACK timeout 或 close-code policy。共享 `channel.rs:38-41` 的 stop 默认实现也不提供
  Gateway 生命周期语义。
- DIVA 对照：`discover_gateway:260-281` 对
  `GET /gateway/bot` 发送 `Authorization: Bot <token>` 并解析 `{url}`；
  `run_gateway:283-326` 使用 15 秒 connect timeout、1 秒起始且上限 60 秒的指数
  backoff；`consume_gateway:328-459` 期待 op 10 Hello，发送 op 2 Identify（token、
  `config.intents`、三项 properties），在已有 session 时构造 op 6 Resume；处理
  READY、RESUMED、MESSAGE_CREATE、op 11 ACK、op 7 `Execution` code
  `gateway_reconnect`，以及清除 session/sequence 后返回 `gateway_invalid_session`。
  首个 Hello 等待是 30 秒，heartbeat 未收到 ACK 时返回 `heartbeat_ack_timeout`；
  `start:942-957` 和 `stop:1029-1037` 共享 cancellation token。
- 现有 fixture/测试与精确结果：
  `tests/fixtures/c5/discord/gateway-frames.json:2-8` 静态列出 op 10、2、0 READY、
  op 1、op 11、op 7、op 9，但它只被
  `channel_adapter_shared_tck.rs:428-458` 当作可解析 JSON。既有
  `adapters::discord::tests::wire_discord_gateway_handles_hello_identify_ready_heartbeat_and_cancel`
  （源码 `discord.rs:1548-1594`）实际由 `spawn_gateway_fixture:1701-1768`
  发送 op 10，接收客户端 op 2 后发送 READY 和 MESSAGE_CREATE，再对客户端 op 1
  回 op 11；它只断言存在 op 2/op 1、收到一条 ingress 和取消后 task 为 `Ok(())`，
  discovery 收到一次 `/gateway/bot`。没有断言 Identify body、heartbeat sequence
  或 op 11 的具体字段。
- 生命周期审计：已有代码覆盖 connect/Hello timeout、backoff、context/local cancel、
  ACK timeout、op 7/op 9 分支；既有 wire evidence 没有实际走这些异常分支。初始
  30 秒 Hello read 没有和 cancellation `select!` 绑定；close code、op 6 Resume
  请求、`RESUMED` 结果、op 7 后的重新连接以及 op 9 后的 cooldown/Identify 都没有
  transcript。
- 最终 disposition：`partial`（`evidence_gap` + `implementation_gap`）。缺少
  RESUME/RECONNECT/INVALID_SESSION/ACK-timeout 的逐帧证据；同时当前 listener 的
  初始 Hello 等待不可立即取消，且没有 close-code/invalid-session cooldown 的可审计
  处理。保留 `ReliabilityResume` 不在冻结矩阵且 blocked 的现状，不升级 capability。

### DC-02 — DM / guild / thread / mention / allowlist / dedup

- 冻结矩阵目标：`IngressText`、`IngressMarkdown`、`IngressThread`、`IngressGroup`、
  `IngressDirect`、`IngressDedupId`（`channel_adapter_shared_tck.rs:227-234`）。
  mention/filter 是群组 ingress 的行为要求，不是单独 capability atom。
- Octos 对照：`crates/octos-bus/src/discord_channel.rs:85-145`
  (`Handler::message`) 的顺序是 bot 过滤、`MessageDedup::is_duplicate`、精确
  `allowed_senders`、附件下载、发送 `InboundMessage`；`DiscordMessage` 的
  `guild_id`/`reply_to` 只进入 metadata（`:96-111`, `:127-140`），没有 thread
  字段或 mention-only policy。`crates/octos-bus/src/dedup.rs:19-74` 定义 60 秒
  TTL/1000 容量及空 ID 规则。
- DIVA 对照：`parse_incoming_message:1074-1145` 解析 message/channel/author ID、
  `guild_id`、`thread.id` 或 type 10-12 推导的 thread、`message_reference`、content、
  attachments 和 `mentions[*].bot`；`handle_incoming:461-492` 依次检查已见 ID、
  bot author、共享 `is_sender_allowed`、可选 guild、guild `mention_only` 和
  `group_reply_allowed_sender_ids`，然后才读取媒体和 admission。配置含义见
  `agent-diva-core/src/config/schema.rs:740-762`；该处注释声称配置 guild 时 DM 仍被
  允许，但 native adapter 的 `message.guild_id.as_deref() != Some(configured_guild)` 条件
  会拒绝 `guild_id=None` 的 DM，这是本行的 implementation gap。Fabric admission deadline 是 2 秒；Busy 映射为可重试的
  `Execution` code `fabric_busy` 并带 `retry_after`，其它失败为 `fabric_admission`。
- 现有 fixture/测试与精确结果：
  `gateway-frames.json` 没有 MESSAGE_CREATE；实际 gateway fixture 的 inline event
  在 `discord.rs:1742-1753`，只覆盖一个 guild message：`id=snowflake-in`、
  `channel_id=channel-1`、`guild_id=guild-1`、thread-1、reply root、bot mention、
  非 bot author。`incoming_message_preserves_thread_and_reply:1329-1338` 单测
  断言 thread/reply；`channel_thread_types_use_channel_id_without_marking_dms_as_threads:
  1341-1355` 只对 synthetic type 11/type 1 做 parser 断言；Gateway wire test
  `1548-1594` 只接收一条 envelope，并断言 chat/thread/message/reply identity。
  既有验证日志记录本组 Discord adapter tests 总计 9 passed，但没有 allowlist、
  bot、mention 或 replay 的独立通过记录。
- 去重/Admission：DIVA 在 `seen` read check 后，只有 Fabric admission 成功才在
  `handle_incoming:558-570` 插入 ID；因此 Fabric Busy 不会提交 dedup，这一点保留为
  正确证据。但 `HashSet` 的 read-check 与 write-insert 不是原子 reserve；并发的同一
  message ID 可以同时通过并重复 admission。它也没有 Octos `MessageDedup` 的 TTL、
  LRU capacity 或失败 `forget` 语义。
- 最终 disposition：`partial`（`evidence_gap` + `implementation_gap`）。缺少真实
  DM/guild allow/deny、bot author、mention-only/exception、重复 MESSAGE_CREATE 和
  Fabric Busy 后重放的 wire matrix；代码还把“任意被标记为 bot 的 mention”视为
  `mentioned_bot`，没有和本 bot ID 比较，且 dedup 不是 TTL/LRU/原子 reserve。已有的
  policy-before-media 与 admission-before-dedup-commit 证据保留。

### DC-03 — Attachments / multipart / typed ingress

- 冻结矩阵目标：入站 `IngressTypedAttachments`（`channel_adapter_shared_tck.rs:233-234`）；
  出站关联 `EgressImage`、`EgressAudio`、`EgressVideo`、`EgressFile`
  （`:239-242`），并受 `EgressText`/`EgressMarkdown`/`EgressChunking`/`EgressReply`
  （`:235-238`）的组合语义约束。
- Octos 对照：`crates/octos-bus/src/discord_channel.rs:104-125`
  (`Handler::message`) 逐个调用 `download_media`，按 attachment ID + extension
  写入本地 media directory，失败只 warning/drop；`:197-226` (`send_with_id`) 对
  `msg.media` 逐个 `CreateAttachment::path`，用同一个 `CreateMessage` content
  调用 `send_files`，返回真实 message ID。`channel.rs:312-324` 规定有文件时直接
  发送，不走文本 chunking。Octos 这里是 path-based `InboundMessage.media`，不是
  DIVA `AttachmentRef`/`ContentPart` seam。
- DIVA 对照：入站 `handle_incoming:490-550` 先按声明 size 和下载后 byte length
  做 25 MiB 上限判断，GET attachment URL，把 `IngressAttachment` 交给
  `AdapterServices.attachments.put:517-531`，再按 MIME 前缀产生 Image/Audio/Video/
  File `ContentPart`；出站 `send_message:602-680` 把 text/card 与 attachment 分开，
  `send_attachments:689-729` 从 AttachmentStore `get` bytes，构造
  `payload_json={"content":""}` 和 `files[index]` multipart，携带 Bot auth；
  reply 只额外放在 `X-Reply-To` header。成功 response 必须为 JSON `{id}`，否则为
  `Execution` code `discord_response`。
- 现有 fixture/测试与精确结果：
  `wire_discord_rest_preserves_reply_embed_and_attachment_receipts:1363-1496`
  的 inline HTTP sequence 是：`POST /channels/channel-1/messages` → 200
  `{"id":"snowflake-1"}`（reply `snowflake-parent` + embed）；`PATCH .../snowflake-1`
  → 200 `snowflake-2`；`DELETE .../snowflake-2` → 204；`PUT .../reactions/%F0%9F%91%8D/@me`
  → 204；`POST .../typing` → 204；最后 multipart `POST /channels/channel-1/messages`
  → 200 `snowflake-attachment`。测试断言 `Accepted` receipt 的这些 ID，以及首个
  JSON reply/embed、最后 multipart 的 `image.png`/`files[0]`。其 `Store::get` 在
  `discord.rs:1279-1287` 返回空 bytes，故没有真实 MIME/bytes 读回断言；只覆盖一
  个 outbound image shape。`rest-responses.json:1-5` 未被该 wire test 消费。
- 失败/边界审计：入站 GET 没有先检查 HTTP status，`response.bytes()` 先完整读取后
  才比较大小；超限或下载后超限会静默跳过，不返回 typed attachment error；GET/读取
  阶段没有 context cancellation select（只有 reqwest client 的 30 秒总 timeout，
  `discord.rs:148-151`）。未验证损坏内容、MIME 不匹配、文件名、多个附件、HTTP
  失败或 store digest mismatch。
- Octos/DIVA 差异：Octos 把 caption/content 与 files 放在同一次 `send_files`；DIVA
  先发 text chunks，再发空 content 的 attachment message。attachment-only reply
  使用的 `X-Reply-To` 不是 `payload_json.message_reference`，现有 fixture 也没有
  断言该 header，因此 reply+attachment 不是已证明的 Discord wire contract。DIVA
  的 `Part::bytes` 只设置 filename，没有把 `AttachmentRef.media_type` 传到 multipart
  part；Image/Audio/Video/File 也没有各自的 wire transcript。
- 最终 disposition：`partial`（`evidence_gap` + `implementation_gap`）。缺少四种
  media kind 的出入站、真实 bounded download/store/读回、MIME/size/corruption/多附件
  和 multipart caption/reply 证据；当前入站 status/完整 body bound/取消语义及附件
  reply wire 仍有实现缺口。不能因共享 AttachmentStore TCK 的一般性 SHA/path 测试
  就把 Discord adapter 的 typed ingress 视为已验证。

### DC-05 — 429 / Retry-After / permission / health

- 冻结矩阵目标：`ReliabilityHealth`（`channel_adapter_shared_tck.rs:249-252`）。
  429/permission 是 transport error contract；`InteractionListening` 不在 Discord
  snapshot，当前应保持 false 并在 transport 前 `UnsupportedCapability`。
- Octos 对照：`crates/octos-bus/src/channel.rs:43-69` 的 typing/listening 默认是
  no-op/fallback，`:241-247` 的 `health_check` 默认返回 `Unknown`；Octos
  `DiscordChannel` 没有自定义 health probe 或 rate-limit parser。其 Discord 专用
  send/edit/delete/reaction/embed 方法在 `discord_channel.rs:197-337` 直接把 Serenity
  结果映射为 `eyre::Result`，源码没有独立 Retry-After/permission receipt 形状。
- DIVA 对照：`parse_response:743-773` 对 HTTP 429 优先读取 JSON body `retry_after`，
  再 fallback 到 `retry-after` header，返回 `AdapterError::RateLimited`；其它非 2xx
  返回 typed `AdapterError::Execution` code `discord_http`，body 截断至 256 字符，
  5xx 才标 retryable。`probe_health:905-928` 对 `GET /gateway/bot` 成功设置
  `Healthy` 并返回 `Accepted` health receipt，失败设置 `Degraded` 并返回
  `Execution` code `health_http`；HTTP transport failure 是 `health_probe`。30 秒
  reqwest client timeout 在 `:148-151`。`ensure_supported:238-258` 先做 channel /
  capability check；`execute:959-974` 对 Listening 返回
  `UnsupportedCapability { InteractionListening }`。
- 现有 fixture/测试与精确结果：
  `wire_discord_rate_limit_and_unsupported_listening_are_explicit:1499-1545`
  的 inline response 是 `POST /channels/channel-1/messages` → 429 body
  `{"retry_after":1.25}`、header `retry-after: 9`；断言结果为
  `RateLimited { retry_after: 1250ms }`。随后发送 Listening command，断言
  `UnsupportedCapability { InteractionListening }`，HTTP spy 总请求仍为 1，证明
  unsupported path 没有第二次网络副作用。`rest-responses.json:4-5` 虽有
  `rate_limited` 和 `{code:50013,message:"Missing Permissions"}` 形状，但没有被
  该测试作为 403 response 使用。
- 失败/边界审计：没有 health success/failure transcript、403 permission response、
  401/malformed body、header-only/malformed Retry-After、连接超时或 stop/reconnect
  health 状态测试。429 在 adapter 内只返回可重试错误，不做本地幂等重试；后续是否
  重试由 runtime command policy 决定，当前没有 Discord-specific wire proof。403
  会得到 `Execution{code="discord_http", retryable=false}`，不是专门的
  permission error code；这属于当前实现的错误分类限制，不能用 fixture 文件名替代
  实际 response transcript。
- 最终 disposition：`partial`（主要为 `evidence_gap`，并保留 permission error
  typed-classification 的 `implementation_gap`）。429 body/header precedence 与
  Listening zero-call 已有精确证据；health、403 permission、超时/重连和 retry
  生命周期尚未证明，当前也没有专门 permission typed code。

## 未重新审计但保留的 DC-04 窄行

DC-04 继续保持既有 `verified` 记录：`execute_edit`、`execute_delete`、
`execute_reaction`、`send_message` 的本地 wire test 在
`discord.rs:1363-1496` 断言 embed/reply POST、PATCH、DELETE、reaction、typing
路径/status 以及真实 response IDs。它不替代本页四个分配行的缺失生命周期和媒体/策略
证据，也不改变 manifest/JSON 的状态。

## 审计结论

DC-01、DC-02、DC-03、DC-05 均为 `partial`；没有把静态 fixture 可解析性、历史通过
记录或代码中未被执行的异常分支当作完整 wire proof。敏感 token 只在源码构造的
Authorization header 中出现，当前 fixture 使用 `snowflake-redacted`、
`session-redacted` 等脱敏值；本次没有产生或记录任何 live credential、signed URL、
私有媒体或完整私有 Discord identifier。
