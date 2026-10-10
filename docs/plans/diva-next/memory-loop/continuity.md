# 持续修复准入与阻塞处理

本文件补充开发开始条件，不修改产品能力、Story 验收依赖、V01–V28 断言或样本门槛。当前状态、负责人和下一动作仍由 README 统一维护。

## 两种准入条件

**修复准入：** 精确代码/依赖与模型输入可追踪；隔离数据根；目标缺陷可复现；目标测试真实执行；修改边界与直接消费者明确。满足这些条件即可定位、写失败回归和实施最小修复，即使原生桌面或其他 Story 的出口仍 Blocked。

**验收准入：** 原 Story 全部直接前置通过，同一重建候选具备所需密封 artifact、平台和模型资源，本 Story 所有必需测试/样本通过，证据 checker 通过。此前的 Blocked 不能改为 Done，跨候选证据不能拼接。

S01 的原生宿主验收不能作为所有 runtime 修复的开始条件。S03 的 reflection/recall 模式也不能阻止只消费 ack、terminal、accepted、canonical 的 S04 采集修复。每项修复仍须读其真实输入契约；某消费者需要的接口或资源缺失时，该修复自身等待，转到不依赖它的工作。

## 连续执行规则

1. 选择 README 的最高优先级且满足修复准入的工作，读取所属 Story 和 Ruling；默认单负责人执行。
2. 保存目标测试首次失败和实际 request/Journal/source/canonical 证据，确认首个错误边界。缺资源不能用 mock backend、SQL 种数据或伪造身份替代正向路径。
3. 实施一个最小修复，运行目标测试和直接受影响回归，记录实际数量、exit code 和原始日志；零匹配、required skip、未知写入效果均不算通过。
4. 聚焦定位可使用已生成的诊断 overlay，但必须注明它不绑定新源码，不作正式验收。变更代码/依赖形成候选时重新 SDK pack/Inspect 和生成 overlay；旧 baseline/evidence 原样保留。产品候选通过受支持的 development/repin 流程重建，不默改原锁定版本。
5. 独立提交代码、证据和未解决风险。原生/live 等仍待资源的出口保留 Blocked；继续下一项满足修复准入的工作。禁止因当前测试绿色把整套 CI 标绿。
6. 当原生和模型资源就绪，在同一冻结候选上按原验收依赖次序复验。修改源码后重置受影响验收；不会自动继承之前候选的通过项。

## 资源与代码问题的处理边界

- GTK/WebKit/libsoup/glib：已通过任务目录下载并提取 Debian 原生开发库，设置局部 pkg-config/运行库路径，支持的 Linux sealed go-host build/test 已通过。不修改系统包或 HOME；Windows UI 仍是独立验收资源。
- Windows 与真实模型：提供测试机/runner 名称、provider/model 和凭据配置入口名称，密钥由环境注入。资源等待不占用代码修复工作序列，也不能用脚本模型计入 live-model 分母。
- INOFY 路径：使用 DIVA/VIVY 已锁定 `v0.0.0-20260930141905-71e2c9bbe47d` 的缓存资源补齐 task-local `../../INOFY`；不升级依赖。已在本环境验证完整 Laputa 63 项通过。
- Mentle CLI dependency sum：先在隔离 modfile/工作树复现并确认已锁定 module graph，补齐缺失的 go.sum 条目，观察 CLI 失败回归 red→green，再跑原模块套件。未验证前不宣称问题已解决。
- Garden process e2e 的模型目录：将真实已锁定模型通过配置/安装入口绑定到独立 profile，确认可用性和 canonical；先复验 memory_unavailable 用例，不从用户 profile 拷贝记忆，也不换假后台。
- `/tmp/.git` 导致的指令测试失败：保持托管 mount；使测试对“存在祖先 Git 根”的场景有明确预期，或在真正没有祖先 Git 根的隔离环境复验。仅换 TMPDIR 到 `/workspace` 同样会遇到其祖先 `.git`，不能算修复。不随意修改生产指令扫描语义。

## 本轮已经执行的环境解阻

在 `/workspace/work/memory-loop` 创建 `INOFY` 符号链接，目标为现有 task-local GOMODCACHE 中精确锁定的 INOFY 模块。未更改产品 go.mod/go.sum、DIVA 锁或上游仓库。

随后从 Laputa 子模块执行 `go test -json ./... -count=1`（使用 task-local Go 1.26.4 与缓存）；实际退出 0，63 个 named test/subtest pass、0 skip、0 fail。原先 missing replacement 失败记录继续保留。该结果解除独立 Laputa 回归缺口，未取得密封宿主或完整记忆闭环验收。

接手后先确认该链接和缓存仍存在；环境重建时下载相同 pin 并重新验证。资源补齐的证据见 `docs/logs/2026-10-memory-loop-unblock/v0.1.0-continuous-repair/`。

## 阻塞卡片与恢复条件

遇到新的阻塞，记录所属 Epic/Story、首次失败日志、精确候选、影响的测试、负责人、可在当前环境执行的下一步与复验命令。按下面规则处理，不把所有未完成开发都归为外部阻塞：

| 类别 | 处理与恢复条件 | 后续工作 |
|---|---|---|
| 工具、目录、依赖、模型安装等环境问题 | 在任务目录补齐锁定资源，保留失败日志，实际复跑成功后解除 | 继续当前修复 |
| 已复现的代码缺陷 | 保存失败回归，修复拥有该契约的模块，复跑受影响测试和规定 CI | 冻结源码、重建候选，再开始下一项 |
| 消费接口或测试观察能力缺失 | 先实现相应 S03/S04 能力；保持真实 App、后台与进程路径 | 转到已有输入的修复，或先补直接前置接口 |
| Windows 真机或真实模型资源 | 只阻塞 S12/S13 及需要这些资源的验收；记明 runner、模型及凭据入口需求 | 持续推进可执行的 S03–S11 |

每个终端会话结束前提交已完成的独立修复，刷新索引和下一动作，并释放本工作树锁。接手人从当前 checkpoint 的实际版本与工具环境恢复。持续修复指可恢复的工作循环；本轮没有部署无人值守服务或定时任务。

## 中断后恢复当前工作

当前开发检查点为 [完整请求与大来源数据包](../../../logs/2026-10-memory-loop-chain/v0.2.0-complete-inference-and-packets/summary.md)。VIVY `e1de33ae` 保留完整 JSON/schema，Laputa `9bc39af` 移除后续阶段不再使用的重复证据；策略 implementation revision 为 `diva-cognitive/v1-review-2`。之前预算、反思观察和进程协议的提交继续保留。最终候选尚未冻结；不要沿用前检查点的 conformance/source hash 宣称新源码通过完整 CI。

1. 检查三工作树的 branch、HEAD、dirty 状态和 LOCK。保留既有工作，在本隔离分支认领具体范围；若环境被重建，先按锁定版本补回工具/模型，而非更换后台。
2. 从 VIVY 工作树执行 `source /workspace/work/memory-loop/tools/environment.sh`，确认 `go version`；UI 安装需要 registry 时使用 `https://registry.npmjs.org/`。不得修改系统目录或用户真实 profile。
3. 按新检查点 verification.md 的命令复跑所需直接消费者，记录 exit code、匹配数、required skip 和 raw log。诊断组合测试与默认生成套件分别运行，不把其中任一结果冒充最终密封候选。
4. 回到 README 的开发序列，选一项有真实输入的缺陷：首次失败→确认 owner/根因→最小修复→直接回归→独立提交→下一项。recall、ACTMEM 等未实现能力仍归本地开发队列。
5. 对提交后失败/未知效果的输入窗口保留 Journal 和回执，不清空数据库、不补造水位、不自动当作失败无副作用。优先在 S06/S10 验证稳定操作身份和恢复路径，再扩大自动反思负载；可同时转到不消费该窗口的测试工作。
6. 本地实现完成后冻结源码，执行规定完整 CI、conformance 复现、重建及身份检查；随后才按原 Story 前置提交正式验收证据。只有需要 Windows runner 或真实 provider/model 配置的测试交给外部资源。
