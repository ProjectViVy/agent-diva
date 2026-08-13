# 2026-08 全量文档整理归档清单

- 批次：`2026-08-docs-corpus-reset`
- 日期：2026-08-13
- 归档范围：`docs/` 中 2026-08-01 以前的日志，以及当前入口中不再作为权威的旧架构、设计、PRD、Sprint、报告、UX、提示词和开发资料。
- 保留范围：全部调研全文、真实方向性决策、8 月日志、当前架构入口。
- 代码范围：未修改任何生产源码、配置或构建文件。

## 当前保留入口

| 分类 | 入口 | 说明 |
| --- | --- | --- |
| 当前架构 | `docs/architecture/README.md` | 以 2026-08-12 至 2026-08-13 决策记录为锚点 |
| 关键决策 | `docs/decisions/README.md` | 提炼仍有方向、范围或安全价值的真实决策 |
| 当前/历史调研 | `docs/research/README.md` | 调研全文保留；Research Hold 不等于删除 |
| 8 月日志 | `docs/logs/README.md` | 2026-08 日志原地保留 |
| 工程参考 | `docs/engineering/README.md` | CONTRIBUTING、上下文和 CHANGELOG |

## 归档目录

| 目录 | 内容 | 文件数 |
| --- | --- | ---: |
| `architecture/legacy/` | 旧架构、治理、Memory、Plan/TODO、AutoDream/Evolution | 26 |
| `legacy-docs/` | 旧设计、计划、PRD、报告、提示词、安全、UX 和旧 docs archive | 53 |
| `legacy-dev/` | 旧开发专题包、开发归档和过往开发资料 | 340 |
| `legacy-batches/` | 本次整理前已存在的 31 个历史归档批次及 7 个遗留根文件 | 1060 |
| `logs/pre-2026-08/` | 2026-08-01 以前的迭代日志 | 1698 |

调研没有被删除或丢入“不可读”归档：

- `docs/research/historical/2026-06-harness-and-reference/`：26 个文件；
- `docs/research/historical/2026-07-08-laputa-memory-history/`：8 个文件；
- `docs/research/historical/2026-08-approval-provider-history/`：4 个文件；
- `docs/research/papers/`：论文及索引。

## ZIP 压缩包

ZIP 是对应归档集合的可移交压缩副本，不包含其他 ZIP，文件数按 ZIP 内非目录条目计算。

| 包名 | 来源目录或用途 | 文件数 | 大小（字节） | SHA-256 |
| --- | --- | ---: | ---: | --- |
| `2026-legacy-architecture.zip` | `architecture/legacy/` | 26 | 69689 | `11761B248A7E3CA60A7C355392201277360B64EFD26F86563FDDBF0BC5D6F89A` |
| `2026-legacy-docs.zip` | `legacy-docs/` | 53 | 290096 | `A26D25B527B26E34F8D6C66865086C64FD68E408BDA6B5F4FCC17FEF65690BE0` |
| `2026-legacy-dev.zip` | `legacy-dev/` | 340 | 1189589 | `8182DF43DE66425D47AB0B88A5C27DED03EE078394C8F7B938A54138DDC6D76C` |
| `2026-preexisting-archive-batches.zip` | `legacy-batches/` | 1060 | 5167146 | `CA19A4CE6ADA34600405715FE7C87E51455585C0978C83CDA8F23635A084BF52` |
| `2026-pre-08-logs.zip` | `logs/pre-2026-08/` | 1698 | 1220606 | `3E971C5A46D3952EB04CDB4B45DDDD6869298E917A56D9727DEC87F705557125` |
| `2026-06-harness-and-reference.zip` | `docs/research/historical/2026-06-harness-and-reference/` 的保留快照 | 26 | 166362 | `2C9FF859DC5E0A75CE103E87F9E85383C2F93FED6949F84389A3A67A9A0FA9E1` |
| `2026-07-08-laputa-memory-history.zip` | `docs/research/historical/2026-07-08-laputa-memory-history/` 的保留快照 | 8 | 55724 | `C30F7343B0AFF361328D52622914C4418773475169C420A74E8A73D784C7789E` |
| `2026-08-approval-provider-history.zip` | `docs/research/historical/2026-08-approval-provider-history/` 的保留快照 | 4 | 27445 | `135523A93238FFCB7B105253C09F83A479279DA877A7B4640E0EABCAFD11519` |

## 完整性规则

- 归档移动不删除原文；原文内容和相对结构保留在归档批次或 `research/historical/`。
- 当前文档不得把归档路径当作当前架构入口；历史文件中的旧相对链接只用于追溯。
- ZIP 不包含 ZIP，避免递归膨胀。
- 8 月日志不压缩到归档批次，作为当前迭代证据保留在 `docs/logs/`。
