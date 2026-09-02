# Telegram：Octos → DIVA 深度扫描报告

> 扫描类型：C5-P2 只读事实扫描。基线为 Octos `5ea987813de4fd2afdd1d78f2106ad2868f0d923`
> (`v2.0.3-rc.9`)；行号以扫描时源码为准，实施前若文件漂移必须重新核验。

每个差距表的 `Decision` 使用 `Port`、`Adapt`、`Retain-DIVA`、`Reject` 或 `Blocked`。

## 证据入口

| 侧 | 文件与定位 |
| --- | --- |
| DIVA transport | `agent-diva-channels/src/telegram.rs`: `Command` 18-32，allowlist 131-145，入站 269-332，命令菜单 426-440，polling 468-668，send/edit/delete 682-760，health 769-773 |
| DIVA policy/assembly | `agent-diva-channels/src/base.rs`: `BaseChannel`/`handle_message` 73-220；`src/manager.rs`: 校验/注册 59-219；`agent-diva-core/src/config/schema.rs`: `TelegramConfig` 725-736 |
| Octos transport | `.workspace/octos/crates/octos-bus/src/telegram_channel.rs`: state/limits 26-66，mention 69-120，download 123-138，send fallback 141-163，commands 184-210，keyboard 213-243，polling/events 261-518，egress 529-631，typing 648-657，edit/delete 700-760，health 769-773 |
| Octos contract/tests | `.workspace/octos/crates/octos-bus/src/channel.rs` 17-253；`tests/api_channel_property.rs` 135-456（thread binding/property）；`book/src/configuration.md` 与 `book-zh/src/configuration.md` |

## 外部 API 与协议行为

Telegram SDK 将请求编码为 `https://api.telegram.org/bot{token}/{method}`；实现 agent 必须在
mock transport 中断言 method、JSON/form 字段和返回 ID，不以 teloxide method 名称作为证据。

| 操作 | Octos 行为 | DIVA 当前 | 目标/决策 |
| --- | --- | --- | --- |
| `getUpdates` long polling | `polling_default`，保存 `last_update_id`，stream 结束后 5s→60s 退避重连 | 468-668 有 dispatcher/polling，但未形成统一退避、dedup 和 admission 合同 | `Adapt`：保留 teloxide，补退避、取消和 Fabric admission fixture |
| `getFile` + file download | `Bot::get_file(file_id)` 后下载 `https://api.telegram.org/file/bot{token}/{file_path}`，单次媒体超时 30s | 旧 handler 仅部分媒体路径；统一 typed attachment 尚未冻结 | `Port`：photo/voice/audio/document 通过 AttachmentStore，失败是 typed error |
| `sendMessage` | HTML；带 `ReplyParameters`；Telegram 解析失败时同一内容纯文本回退 | 682-741 已有 HTML/reply/fallback | `Adapt`：保留回退，但 receipt 指向真实成功 message ID |
| `sendVoice`/`sendAudio`/`sendDocument` | 媒体分流；第一项可带 caption，caption 截断 1024 | DIVA 有基础发送但缺少统一 MIME/大小/部分失败语义 | `Port`：每一项上传/失败/receipt 都必须可观测，不得 log-and-skip |
| `sendChatAction` | `Typing`、`RecordVoice`，按 chat 维持可取消任务 | DIVA 已有 typing 任务；listening 语义不统一 | `Port`：typing/listening 纳入能力合同，取消时清理所有任务 |
| `answerCallbackQuery` | callback 必须 ACK；callback 转成独立 inbound 事件，附 callback_data/message_id | DIVA 有 callback 分支但 metadata 契约较弱 | `Port`：callback 是独立事件，保留原始 callback/message ID |
| inline keyboard | metadata 形如 `{"inline_keyboard":[[{"text":"Label","callback_data":"s:topic"}]]}` | DIVA 已有解析/发送 | `Adapt`：冻结 schema、长度限制和恶意 callback 校验 |
| `editMessageText` / `deleteMessage` | 通过 chat_id + message_id；edit 可带 keyboard | DIVA 682-760 已有路径 | `Port`：绑定原始 receipt；不允许跨 chat 编辑 |
| `getMe` | `health_check` 的 authenticated probe | DIVA 769-773 已有 | `Retain-DIVA`：接入统一 health 状态，不宣称 heartbeat/resume |
| Bot commands | Octos 注册 `/new`、`/s`、`/sessions`、`/back`、`/delete` 等 menu | DIVA 当前 `/start`、`/reset`、`/help`、`/stop` | `Adapt`：保留 DIVA 现有语义，逐项决定是否暴露 Octos 命令；所有命令要有权限和 fixture |

## 入站身份、群聊和权限

- DM 无需 mention；群聊仅在 @bot、reply-to-bot 或 command 时响应。
- `sender_id` 必须保留 Telegram user ID、username/compound ID；`chat_id` 是会话地址；
  `message_id` 是消息关联 ID；forum topic 映射到 `thread_id`。
- 空 `allow_from` 按 DIVA 可执行代码允许所有来源；非空支持 `123|username` compound 匹配。
- 未授权事件必须在下载图片/文件前拒绝；外部 callback 不得携带 owner context。
- command 不得绕过 allowlist；群聊 mention 去除只影响模型文本，不得修改原始审计内容。

## 媒体与图片识别

Octos 解析 photo（选最大尺寸）、voice、audio、document；DIVA 目标还需明确 video/sticker 是否
支持。每个附件必须记录 source channel、platform message ID、sender、file name、declared MIME、
大小和 SHA-256，并写入 `ChannelAttachmentStore`。之后才可进入 agent context/provider vision gate。

目标行为：

1. allowlist 和群聊 policy 通过后才调用 `getFile`/下载。
2. 下载超时、404、超限、MIME 不匹配都返回附件级 typed failure；不能替换成空文本成功。
3. 图片识别由 provider capability 决定；不支持 vision 时返回明确错误/降级说明。
4. 不向 Fabric 发布绝对路径、`file` URL 或无限 base64。

## 线程、去重与生命周期

- 以 update/message ID 去重；标记只在 bounded Fabric admission 成功后提交。
- reply-to 只使用 command 携带的冻结 message ID，不从“最近一次聊天”推断。
- polling listener 必须一直运行到取消或明确 transport failure；禁止 spawn 后立即返回。
- 重连退避、typing 子任务、媒体下载、health probe 都必须受 shutdown token 约束。
- 参考 Octos `api_channel_property.rs`，增加不同会话并发下 thread/reply/callback 不串线的属性测试。

## 当前差距与实施决策

| 能力 | DIVA 当前证据 | Octos 证据 | 决策 | 实施验收 |
| --- | --- | --- | --- | --- |
| 文本/caption 入站 | `telegram.rs` 269-332、550-668 | `telegram_channel.rs` 317-334 | `Adapt` | text/photo-caption fixture |
| 群聊 mention/reply gating | 现有实现分散，配置未统一 | 69-98、860+ tests | `Port` | DM/group/mention/reply/command matrix |
| typed media 入站 | legacy `media: Vec<String>` | 317-411 | `Port` | photo/voice/audio/document + timeout/size |
| callback query | 基础 metadata 分支 | 449-490 | `Port` | ACK、callback event、权限、重复 callback |
| inline keyboard | 已有解析 | 213-243、862+ tests | `Adapt` | schema invalid/row/length fixture |
| reply/send ID | 682-741 | 141-163、529-631 | `Port` | request capture + real response ID |
| edit/delete | 700-760 | 700-760 | `Port` | bound chat/message checks |
| typing/listening | typing 已有，listening 不完整 | 648-657 | `Adapt` | cancellation/no detached task |
| reconnect/dedup | 部分存在 | 272-306、503-517 | `Port` | duplicate update and stream end |
| health | `get_me` | 769-773 | `Retain-DIVA` | healthy/auth failure/degraded mapping |
| approval | DIVA governance | Octos 无对应治理 | `Retain-DIVA` | Manager/Sandbox approval identity/expiry |

## 后续实现 agent 的顺序

1. 先冻结 envelope 的 message/thread/reply/callback/attachment 字段。
2. 在共享层接入 permission-before-download、bounded admission 和 receipt。
3. 适配 mention、媒体下载、callback、keyboard、send/reply/edit/delete。
4. 把 `/stop` 从硬编码 localhost 控制面迁移为 DIVA Manager command binding。
5. 完成 TCK 后再进入 C5-I；未覆盖的操作必须在发送前返回 `UnsupportedCapability`。
