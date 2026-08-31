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

## C5-P2 evidence scans

The specification files above define the target contract. The following independently scanned
fact packs define the evidence behind each endpoint and migration decision:

- [`telegram-scan.md`](telegram-scan.md)
- [`discord-scan.md`](discord-scan.md)
- [`feishu-scan.md`](feishu-scan.md)
- [`dingtalk-scan.md`](dingtalk-scan.md)
- [`email-scan.md`](email-scan.md)
- [`qq-scan.md`](qq-scan.md)

## C5-I Gate 2 implementation evidence

These notes map the native adapter commits to the frozen scan and record which
capabilities are implemented versus still awaiting C5-V wire proof:

- [`telegram-gate2.md`](telegram-gate2.md)
- [`discord-gate2.md`](discord-gate2.md)
- [`feishu-gate2.md`](feishu-gate2.md)
- [`dingtalk-gate2.md`](dingtalk-gate2.md)
- [`email-gate2.md`](email-gate2.md)
- [`qq-gate2.md`](qq-gate2.md)

An implementation worker must read both files for its channel. A target capability may not be
claimed from the specification alone; it needs the scan row plus an evidence fixture.

All workers must preserve explicit correlation, bounded admission, truthful receipts, typed
unsupported behavior, secret redaction, and cancellation-aware listener shutdown.
