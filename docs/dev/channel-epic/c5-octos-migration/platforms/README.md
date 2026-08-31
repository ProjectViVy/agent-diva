# Platform migration specifications

Each file is an implementation contract, not a loose research note. A platform worker owns only
its adapter file, platform fixtures/tests, and the matching specification. Shared interface or
matrix changes go through `../decision-requests.md` and the Lead.

- [`telegram.md`](telegram.md)
- [`discord.md`](discord.md)
- [`feishu.md`](feishu.md)
- [`dingtalk.md`](dingtalk.md)
- [`email.md`](email.md)
- [`qq.md`](qq.md)

All workers must preserve explicit correlation, bounded admission, truthful receipts, typed
unsupported behavior, secret redaction, and cancellation-aware listener shutdown.
