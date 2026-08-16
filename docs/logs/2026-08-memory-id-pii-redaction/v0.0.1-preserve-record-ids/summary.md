# 记忆记录 id 不再被脱敏层截断

- 日期：2026-08-17
- 切片：工具结果里的 BML 记录 id 被 PII 脱敏成不可操作的残片
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-memory-id-pii-redaction/v0.0.1-preserve-record-ids/`

## 目标

模型从 `memory_list` / `memory_add` / `memory_search` 拿到的 `id` 必须是可直接用于
`memory_update` / `memory_remove` 的原始记录 id。

## 原因

BML 新记录 id 形如 `memory-{timestamp_micros}-{digest12}`。2026 年的微秒时间戳是
16 位、以 `17` 开头。工具结果统一走 `sanitize_tool_output` → `redact_pii`：

- `phone_cn` 原先没有词边界，会吃掉中间 11 位（`1[3-9]\d{9}`），所以**每条** id
  都会变成 `memory-[REDACTED:Phone]…`。
- 即便补上词边界，16 位时间戳仍可能通过 Luhn，被当成银行卡/信用卡。

记录还在权威库里，只是模型拿到的不是能回写的原始 id。换一条内容不同的记录也会一样，
因为误伤来自时间戳，不是内容 digest。

## 改动

- 手机号匹配加上词边界，避免从更长数字串里切出电话。
- `redact_pii` 先保护 BML / checkpoint id 跨度，再套用电话、卡号等规则。
- 覆盖当前 `memory-{micros}-{hex}`、tombstone、legacy/laputa 导入 id、
  `session-checkpoint-*`。

## 未做

- 没有改 id 生成格式。旧记录继续用十进制微秒时间戳。
- 没有重启用户正在跑的 gateway / 桌面进程；要看到修复必须重启加载新二进制。
