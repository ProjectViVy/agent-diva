# DIVA 记忆闭环验证 — Epic–Story 计划包

版本：2026-10-10。范围：详细行动计划与执行交接。2026-10-09 已开始开发；执行分支 `feat/memory-loop-verification-20261009`。

> 执行采用 Superpowers `executing-plans`，按 Story 的验收前置 gate 推进；已复现缺陷的开发开始条件见 continuity.md。默认一位负责人顺序执行；可并行的依赖关系不等于已授权多个 agent 同时编辑。

## 接手人从这里开始

1. 阅读 [requirements.md](requirements.md)：目标、源码基线、架构约束和已经发现的证据缺口。
2. 阅读 [contracts.md](contracts.md)：测试夹具接口、证据结构、超时和崩溃切点。
3. 修复工作先按 [持续修复准入](continuity.md)选择可执行项，再读所属 Epic/Story；正式验收仍要求下表直接前置通过。
4. 按 [runbook.md](runbook.md)准备隔离环境与命令；按 [verification.md](verification.md)判断结果。
5. 执行完成后将 commit 和证据写入本页状态表；不能仅勾选步骤来宣称产品通过。

本 README 是状态、依赖和排期的唯一人工维护入口；`package-map.json` 是与表格一致的机器校验镜像，无独立状态。其他总索引仅链接到本包。

## Epic 总览

| Epic | 目标 | Story 数 | 基础人日 |
|---|---|---:|---:|
| [E01](epics/E01.md) 可复现基础与真实存储 | 得到可写可读的真实后台和可复用证据工具。 | 3 | 2 |
| [E02](epics/E02.md) 经历采集、活动整理与反思 | 用户事实进入持久来源，并经活动整理、反思与治理形成可追踪结果。 | 4 | 2.5 |
| [E03](epics/E03.md) 记忆回用与生命周期 | 重启后的新会话真正使用记忆，纠正和遗忘同样生效。 | 2 | 1.5 |
| [E04](epics/E04.md) 故障恢复与权限隔离 | 在未知结果、崩溃及不可信输入下保持数据和授权正确。 | 2 | 2 |
| [E05](epics/E05.md) 桌面与真实模型验收 | 同一候选在 Windows 产品及真实模型上取得可复查证据。 | 2 | 2.5 |

<a id="story-status"></a>
## Story 状态与依赖

首轮 S01–S03 已开发准备工作，但前置/出口门槛未全部满足；S04 已有诊断缺陷，尚未进入正式验收。包结构检查通过不等于 Story Ready。Ready 需执行范围明确、前置 gate 满足且资源可用；Blocked 必须给阻塞原因/负责角色/下一动作；Done 必须有本候选的验收证据。

| Story | Epic | 结果 | 直接前置 | 人日 | 状态 | 证据/提交 |
|---|---|---|---|---:|---|---|
| [S01](stories/S01.md) | E01 | 固定版本、构建与隔离环境 | 无 | 0.5 | Blocked | [环境/产物 gate](../../../logs/2026-10-memory-loop-verification/S01/summary.md)；`b2d8b0e9`；[Linux 开发候选与解阻](../../../logs/2026-10-memory-loop-repair/v0.2.0-source-and-environment/summary.md)，正式资源 gate 仍待 |
| [S02](stories/S02.md) | E01 | 真实 Mentle 启动与读写回执 | S01 | 1 | Blocked | [真实后台准备](../../../logs/2026-10-memory-loop-verification/S02/summary.md)；Laputa `5007f62f`，S01 未通过 |
| [S03](stories/S03.md) | E01 | 统一测试夹具与证据校验 | S01 | 0.5 | Blocked | [夹具/校验器准备](../../../logs/2026-10-memory-loop-verification/S03/summary.md)；[反思与复用 Restart 开发回归](../../../logs/2026-10-memory-loop-chain/v0.1.0-reflection-and-restart/summary.md)，VIVY `cd080708`；[完整开发证据导出](../../../logs/2026-10-memory-loop-chain/v1.4.0-development-artifact-recorder/summary.md)，VIVY `8d3d3fa7`；正式 schema/最终候选 gate 仍待 |
| [S04](stories/S04.md) | E02 | 用户对话采集与可信来源 | S02, S03 | 1 | Planned | [MEM-S04-01 诊断失败](../../../logs/2026-10-memory-loop-verification/S04/defects.md)；[来源修复及回执恢复](../../../logs/2026-10-memory-loop-repair/v0.2.0-source-and-environment/summary.md)已验证，前置/完整 gate 未通过 |
| [S05](stories/S05.md) | E02 | ACTMEM 连续性与归档 | S04 | 0.5 | Planned | 开发证据见增量；正式矩阵待 |
| [S06](stories/S06.md) | E02 | 自动反思与普通记忆效果 | S04, S05 | 0.5 | Planned | [自动反思/预算](../../../logs/2026-10-memory-loop-chain/v0.1.0-reflection-and-restart/summary.md)；[完整推理请求及大来源数据包修复](../../../logs/2026-10-memory-loop-chain/v0.2.0-complete-inference-and-packets/summary.md)，VIVY `e1de33ae` / Laputa `9bc39af`；[自动门槛开发证明](../../../logs/2026-10-memory-loop-chain/v1.2.0-automatic-trigger-gates/summary.md)，VIVY `b2e937b1`；四项 race 通过零跳过；完整 case/前置 gate 未通过 |
| [S07](stories/S07.md) | E02 | 人格审阅与冻结会话边界 | S06 | 0.5 | Planned | [模型／调用者人格权限开发证明](../../../logs/2026-10-memory-loop-chain/v1.5.0-persona-authority-denials/summary.md)，VIVY `63ce624e`；八项正负向通过；完整矩阵／前置待 |
| [S08](stories/S08.md) | E03 | 新会话召回及因果对照 | S06, S07 | 1 | Planned | [v3.0 开发收口记录](../../../logs/2026-10-memory-loop-chain/v3.0.0-s08-s11-closeout/summary.md)：重启后真实新会话召回、三 profile/六负对照及纠正删除模型输入通过；V19 WORLD/ACTMEM 与显式工具读取边界、正式前置仍待 |
| [S09](stories/S09.md) | E03 | 纠正、遗忘与防止旧记忆复活 | S08 | 0.5 | Planned | [v3.0 开发收口记录](../../../logs/2026-10-memory-loop-chain/v3.0.0-s08-s11-closeout/summary.md)：纠正后新会话读取修订事实；删除后重启并三次重投旧 run，canonical 不变、墓碑不复活、新会话不含旧事实；正式前置/完整矩阵仍待 |
| [S10](stories/S10.md) | E04 | 进程崩溃、未知结果与幂等 | S06, S09 | 1 | Planned | [C01 终态-before-Capture 10 次](../../../logs/2026-10-memory-loop-chain/v2.3.0-s10-c01-terminal-before-capture/summary.md)，VIVY `160349ca`；[C02 receipt-before-ACK 10 次](../../../logs/2026-10-memory-loop-chain/v2.2.0-s10-c02-capture-before-ack/summary.md)，VIVY `fc78d0c6`；[C03](../../../logs/2026-10-memory-loop-chain/v2.4.0-s10-c03-canonical-before-caller-receipt/summary.md)、[C04](../../../logs/2026-10-memory-loop-chain/v2.5.0-s10-c04-partial-effect-batch/summary.md)、[C05](../../../logs/2026-10-memory-loop-chain/v2.6.0-s10-c05-effects-before-watermark/summary.md)、[C06](../../../logs/2026-10-memory-loop-chain/v2.7.0-s10-c06-canonical-before-index/summary.md) 各有 10 次开发 race 样本；未知 effect/半批恢复和同候选 gate 仍待 |
| [S11](stories/S11.md) | E04 | 后台退化、scope 隔离和不可信记忆 | S09 | 1 | Planned | [v2.9 host-bound cursor 检查点](../../../logs/2026-10-memory-loop-chain/v2.9.0-s11-hostbound-cursors/summary.md)；[v3.0 开发收口记录](../../../logs/2026-10-memory-loop-chain/v3.0.0-s08-s11-closeout/summary.md)：ordinary injection 通过；race 延迟根因确认为 hybrid search；race-only 5s 测试预算下数据边界通过，默认 750ms 超时安全降级单独通过；backend recovery/workspace A/B 仍未执行 |
| [S12](stories/S12.md) | E05 | Windows 密封桌面的完整记忆流程 | S10, S11 | 1.5 | Planned | 未执行 |
| [S13](stories/S13.md) | E05 | 真实模型评估与接手验收报告 | S12 | 1 | Planned | 未执行 |

## 当前执行交接

[首轮执行记录与解阻顺序](../../../logs/2026-10-memory-loop-verification/handoff.md)保存三个仓库的本地提交、真实测试数量、失败记录和命令。所有 S01–S03 当前状态为 **Blocked**，不是 Done；资源补齐和夹具实现完成后重新执行 gate。

- S01：本地 GTK/WebKit 等依赖已补齐，支持的密封 Linux test 与 development build 已通过；完整候选及 Windows 验收仍待。负责人：集成负责人；下一步：使用本地工具环境持续重建，原始锁和历史 baseline 保持不变。
- S02：独立 Laputa 63 pass；Mentle 536 pass、0 skip（含 MCP 8 项）；Garden 全套和进程 e2e 已复验通过。负责人：存储集成负责人；下一步：在最终同一候选中复验正式 case 与证据，不计为完整 Story Done。
- S03：25 项 Python 回归和既有 composition/身份校验已交付；reflection、reflected、复用跨进程 Restart 及关闭/重启后的 action 路径已通过开发回归，组合 10 pass / 0 skip。负责人：VIVY 测试集成负责人；下一步：补 recall，冻结新源码后重建同一候选并重新验收；历史 overlay 仅作诊断。
- S04：用户来源丢失已修复并用随机事实、实际请求和 canonical 验证；角色来自实际持久 envelope。负责人：采集链路负责人；继续负向、旧来源补写与 V05–V09 正式验收；不把单项修复记为 Story Done。
- S06/S10：累计预算、推理 JSON/schema 截断及反思包重复携带证据已通过失败回归修复；超过 4 KiB 的中文来源也取得完整 actual model input、canonical 记忆、回执与水位证明。负责人：runtime/恢复链路负责人；下一步：其他输入边界、disabled/busy/no-input policy 与任意提交后失败的窗口恢复；不要当作完整矩阵或崩溃幂等证明。

## 当前可连续执行的工作序列

以下是开发投入顺序；既有 W1–W11 表及依赖图继续表示正式验收顺序，不将修复准备计为 Story Done。负责人是执行角色，未自动分配多个 agent。

| 顺序 | Epic / Story | 当前可推进动作 | 阻塞影响及负责角色 |
|---|---|---|---|
| 1 | E02 / S04 | 用户来源及旧收据重投已修复；继续剩余 V05–V09 和旧来源补写验证 | 已有 real App/canonical 和两进程来源证明；继续开发，不等 Windows/live |
| 2 | E01 / S02、S03 | 复用 Restart、reflection/reflected 已验证；补 recall 和最终源码绑定候选 | 测试负责人继续补缺失输入/观察能力，不等外部验收 |
| 3 | E02–E04 / S05–S11 | 推理截断/重复数据包已修复；优先提交后失败窗口和剩余边界，再推进 ACTMEM、召回、生命周期及权限矩阵 | 已有普通/大来源反思证明；未解决窗口已保留原 run/水位并公开原因；并发状态已修复；继续原操作恢复和完整崩溃矩阵，领域约束和正式前置不降级 |
| 外部资源跟踪 | E01 / S01、E05 / S12–S13 | 提供 Windows runner 和真实模型配置；Linux 原生构建已解阻 | 环境维护者提供剩余外部资源；限制相应验收，不冻结可执行代码修复 |

当前增量和本地复跑入口见 [canonical 来源检查点](../../../logs/2026-10-memory-loop-chain/v0.5.0-canonical-provenance/summary.md)、[并发状态检查点](../../../logs/2026-10-memory-loop-chain/v0.4.0-state-concurrency/summary.md)、[未解决窗口/字面路径检查点](../../../logs/2026-10-memory-loop-chain/v0.3.0-recovery-and-literal-path/summary.md)及 [完整请求/大来源检查点](../../../logs/2026-10-memory-loop-chain/v0.2.0-complete-inference-and-packets/summary.md)，[反思/Restart](../../../logs/2026-10-memory-loop-chain/v0.1.0-reflection-and-restart/summary.md)与此前完整 CI/环境证明仍保留。具体解阻方式、会话恢复、候选重建与禁止事项见 [continuity.md](continuity.md)。全部完成门槛保持原定义；没有新增完成比例或日期承诺。

## 排期：依赖批次

| 批次 | Story | 可交付里程碑 | 批次人日 |
|---|---|---|---:|
| W1 | S01 | 固定版本与环境清单 | 0.5 |
| W2 | S02、S03 | 真实存储 + 复用夹具/证据校验 | 1.5 |
| W3 | S04 | 用户事实进入持久来源 | 1 |
| W4 | S05 | ACTMEM 连续性 | 0.5 |
| W5 | S06 | 自动反思与真实效果 | 0.5 |
| W6 | S07 | 人格审阅与冻结边界 | 0.5 |
| W7 | S08 | 跨会话记忆因果证明 | 1 |
| W8 | S09 | 纠正/遗忘生效 | 0.5 |
| W9 | S10、S11 | 崩溃、退化和权限门槛 | 2 |
| W10 | S12 | Windows 密封产品验证 | 1.5 |
| W11 | S13 | 真实模型与验收报告 | 1 |

基础验证 **10.5 人日**；修复预留 **3–5 人日**；总预算 **13.5–15.5 人日，排程可按 14–16 人日预留**。相比旧版 10 人日，新增 0.5 人日用于明确的共享夹具和证据校验交付。时间为单负责人有效投入，不是日历承诺；模型资源、Windows 和凭据等待单列。S01/S02 后复估。并行批次也按人员总投入计算，不能直接把人日相加当成并行后的历时。

关键路径：S01 → S02/S03 中较慢的验收 → S04 → S05 → S06 → S07 → S08 → S09 → S10/S11 中较慢的验收 → S12 → S13。共享文件有冲突时同一批次串行处理。

## 旧任务映射

| 旧任务 | 新 Story |
|---|---|
| M0 | S01 |
| M1 | S02 |
| 新增复用基础 | S03 |
| M2 | S04 |
| M3 | S05、S06、S07 |
| M4 | S08、S09 |
| M5 | S10、S11 |
| M6 | S12 |
| M7 | S13 |

原 V01–V28 编号保留，每项有唯一主责 Story；Windows 对前序核心链路做产品复验。原单文件计划改为跳转入口，不再维护另一套排期。

## 计划包完成标准

- 5 个 Epic、13 个独立 Story、共享契约、命令手册和验收矩阵均存在。
- 每个 Story 有范围、文件、输入输出、测试断言、步骤、命令、Done/Blocked 和交接要求。
- 依赖无环、28 个场景无遗漏、工期算术及引用路径通过静态检查。
- 本包无产品通过声明；本地提交与产品执行/推送分开。

## 计划包自检

在 DIVA 仓库根执行：

```sh
python3 docs/plans/diva-next/memory-loop/check_package.py --workspace /workspace
```

`--workspace` 指向包含 agent-diva、agent-vivy、laputa 的目录。检查依赖图、索引/机器镜像、场景映射、链接、现有源码路径和工期；它不会运行产品测试。

开发增量：[终态来源与策略](../../../logs/2026-10-memory-loop-chain/v0.6.0-terminal-and-policy/summary.md)，VIVY `2247c39a`。实际完成/失败/取消、12,057 字节中文来源及特殊路径通过；disabled/无新输入/no-change 在两个实际后台间隔内无额外推理。组合23通过零跳过；仅为开发证据，S04/S06 完整矩阵与正式候选仍待。继续本地 ACTMEM/召回/恢复/隔离工作。

开发增量：[作用域 Work 与读取预算](../../../logs/2026-10-memory-loop-chain/v0.7.0-scoped-work/summary.md)，VIVY `d2f751cc` / Laputa `b4e266c`。实际整理请求拿到现有 Work，前台 prompt 无自动 ACTMEM；原生69/488及组合24通过零跳过。`MEM-S05-01` 仍红：两个实际回合后 Pulse/Recap 均为空，需本地产线接线及归档设计，不能以外部 Windows/live 等待代替。完整 Story/候选仍待。

开发增量：[人格审阅与 Mission 效果边界](../../../logs/2026-10-memory-loop-chain/v0.8.0-persona-and-mission/summary.md)，VIVY `6b35f604` / Laputa `db46181`。`MEM-S07-01` 实际红→绿：Mission 变更不再推进过期窗口；人工写入与效果检查/提交共享短时宿主门。人格拒绝/批准及真实进程重启后的新旧 Frozen Core 已验证。组合30、Mission race3、owner race12、Garden492通过零跳过；仅为开发证据，完整 S07/候选及其他本地工作仍待。

开发增量：[公开记忆生命周期与采集重投递](../../../logs/2026-10-memory-loop-chain/v0.9.0-lifecycle-and-replay/summary.md)，VIVY `7a1086a5` / Laputa `db46181`。纠正、删除、真实重启与原回执查询通过；三次受控 ObserverHost 重投递保留原来源，变更事件内容被拒绝。组合32通过零跳过，默认 App133通过22条件跳过。无产品修改；不代替 S08 后续模型输入或 S10 崩溃矩阵；Pulse/Recap 红灯仍开放。

开发增量：[原生普通召回与证据读取安全](../../../logs/2026-10-memory-loop-chain/v1.0.0-native-recall-and-read-safety/summary.md)，VIVY `9cbed3ea` / Laputa `4faf95a`。三 profile 实际新进程召回、六个空／关闭对照及纠正／删除后模型输入通过；串行组合45通过零跳过。原生缓存、Observer 启动、embedding 关闭和证据混版缺陷已分别修复。实际降级 race1通过；正向冷查询 race 超时、早先关闭超时和一次 RPC 失败均保留。默认163通过26条件跳过；仅为开发证据，完整候选与 Story 仍待。继续本地 Pulse/Recap、Agent 显式工具边界、原操作恢复及退化矩阵。

开发增量：[终态活动与删除归档](../../../logs/2026-10-memory-loop-chain/v1.1.0-terminal-activity-and-archive/summary.md)，VIVY `1c0ef749` / Laputa `4da565e`。实际 Pulse/Recap、进程重启 ID/来源、已结束及运行中取消会话删除归档、旧 Work 冲突通过；组合15、原生120／race104、运行时 race44、Observer race12、ACTMEM race27通过零跳过。投影未完成水位、原始回执恢复和无需新投递的 worker 重试已接通；完整候选、崩溃／并发／隔离矩阵及正式 Story 仍待。继续本地 S03–S11；Windows/live 等待不阻断本地工作。

## Operator recovery development checkpoint

[V1.6 ownership fence](../../../logs/2026-10-memory-loop-chain/v1.6.0-operator-recovery-ownership/summary.md), VIVY `189a9f1b`: public operator recovery now uses the existing busy ownership fence. Actual App race regression 1 pass/0 skip; RPC race regression 220 pass/1 Windows skip; both observed exit0. Formal S10 remains Planned and final same-candidate gates remain pending. Next local increment: actual protected Tool output and assistant exclusion from captured user provenance.

## Development checkpoint v1.7.0-protected-tool-provenance

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v1.7.0-protected-tool-provenance/summary.md), VIVY `2ca5798d`. Formal Story states and final candidate gates stay pending. Next local work: Actual native projection failure, archive preservation and recovery; shared response evidence and remaining local queue.

## Development checkpoint v1.8.0-native-fault-and-archive-retry

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v1.8.0-native-fault-and-archive-retry/summary.md), VIVY `a4891f1c`. Formal Story states and final candidate gates stay pending. Next local work: Complete actual request/response recording, manual trigger override and remaining local gates.

## Development checkpoint v1.9.0-request-response-evidence

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v1.9.0-request-response-evidence/summary.md), VIVY `15d3e469`. Formal Story states and final candidate gates stay pending. Next local work: Validate explicit manual trigger against busy and interval gates; continue S03–S11 local repairs.

## Development checkpoint v2.0.0-manual-trigger-override

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.0.0-manual-trigger-override/summary.md), VIVY `15d3e469`. Formal Story states and final candidate gates stay pending. Next local work: Continue the local S03–S11 queue; prioritize S10 operation recovery and S04/S05/S11 crash, scope, and native-fault matrices.

## Development checkpoint v2.1.0-s03-dev-candidate

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.1.0-s03-dev-candidate/summary.md), VIVY `15d3e469`. Formal Story states and final candidate gates stay pending. Next local work: Continue local S04/S05/S10/S11 gaps; keep S08 Agent WORLD/ACTMEM Tool-only design at its review gate, then finish final CI and same-candidate evidence.

## Development checkpoint v2.2.0-s10-c02-capture-before-ack

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.2.0-s10-c02-capture-before-ack/summary.md), VIVY `fc78d0c6`. Formal Story states and final candidate gates stay pending. Next local work: Continue S10 with exact C01 and C03–C06 handshakes; verify unknown-effect and partial-batch recovery. Keep formal S10 pending until all six cutpoints × 10 samples and same-candidate gates pass.

## Development checkpoint v2.3.0-s10-c01-terminal-before-capture

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.3.0-s10-c01-terminal-before-capture/summary.md), VIVY `160349ca`. Formal Story states and final candidate gates stay pending. Next local work: Continue S10 with C03–C06 exact handshakes and operation-level unknown/partial-effect recovery. C01 and C02 each have 10 developer race samples; keep formal S10 pending until remaining cuts and same-candidate gates pass.

## Development checkpoint v2.4.0-s10-c03-canonical-before-caller-receipt

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.4.0-s10-c03-canonical-before-caller-receipt/summary.md), VIVY `03bfe793`. Formal Story states and final candidate gates stay pending. Next local work: Continue S10 with C04 multi-effect partial success; keep the Story Planned until C04–C06, unknown and partial recovery, and same-candidate gates are verified.

## Development checkpoint v2.5.0-s10-c04-partial-effect-batch

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.5.0-s10-c04-partial-effect-batch/summary.md), VIVY `70cc109c`. Formal Story states and final candidate gates stay pending. Next local work: Continue S10 with C05 effects-complete before watermark persistence, then C06 canonical commit before derived-index completion; keep same-candidate acceptance gates pending.

## Development checkpoint v2.6.0-s10-c05-effects-before-watermark

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.6.0-s10-c05-effects-before-watermark/summary.md), VIVY `0cd1b8f7`. Formal Story states and final candidate gates stay pending. Next local work: Continue S10 with C06 canonical memory committed before derived-index completion; keep final source-sealed and same-candidate acceptance gates pending.

## Development checkpoint v2.7.0-s10-c06-canonical-before-index

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.7.0-s10-c06-canonical-before-index/summary.md), VIVY `1489825d`. Formal Story states and final candidate gates stay pending. Next local work: Continue S10 unknown-effect and partial-effect recovery cases, then complete remaining same-candidate and source-sealing gates; keep S10 Planned until every required case passes.

## Development checkpoint v2.8.0-s11-scope-isolation

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.8.0-s11-scope-isolation/summary.md), VIVY `382525d9`. Two actual App cases pass in the ordinary test run; scope isolation also passes under race, while memory recall is not observed in that race run and the race injection path reports a material-read failure. Mentle card search does not mint pagination cursors, so valid foreign-cursor binding is unverified. Backend recovery and the S09 prerequisite remain open; S11 stays Planned.

## Development checkpoint v2.9.0-s11-hostbound-cursors

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v2.9.0-s11-hostbound-cursors/summary.md), Laputa `6c2bf3c1`, VIVY `9dd8150e`. Mentle/Garden pagination, opaque host-bound cursors, and deterministic score tie ordering pass normal and race tests. The actual App ordinary scope/injection cases pass; a valid A cursor is rejected by B in ordinary and scope-race runs. The race scope sample did not invoke ContextHost, and injection race again returned `material read failed`. Backend recovery, workspace A/B, and S09 remain open; S11 stays Planned.

## Development checkpoint v3.0.0-s08-s11-closeout

[Preserved evidence](../../../logs/2026-10-memory-loop-chain/v3.0.0-s08-s11-closeout/summary.md), VIVY/Laputa commits recorded in the checkpoint metadata. S08/S09 targeted actual-App checks pass, including three observer redeliveries after tombstone; S11 ordinary injection and default-budget safe degradation pass. Race injection reaches the model only under a race-only 5s test seam because native card search takes about 2–3s under instrumentation; the production default remains 750ms and its timeout behavior is separately verified. V19, backend recovery, workspace A/B, S01–S03 formal gates and same-candidate acceptance remain open; all Stories stay Planned/Blocked.
