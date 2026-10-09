# 执行与交接手册

## 1. 接手检查

- 读当前仓库及修改目录的 AGENTS、requirements、contracts、所属 Epic 和 Story。
- 检查三个仓库 SHA、dirty state、LOCK；保留无关修改。执行工作建隔离分支/checkout，不覆盖原始锁定基线。
- 只在 README 维护当前 Story 状态、担当角色、证据路径和提交；Epic/Story 文件的步骤可以勾选，但不另维护状态表。
- 未接到执行指令时本包仅作为计划。开始后按 `superpowers:executing-plans` 原生顺序执行；依赖图不自动授权子代理。

## 2. 环境和构建入口

以下路径变量由 S01 写入 environment.md。命令中的 `/absolute/...` 必须替换为记录的实际路径。配置的 data_dir、workspace、models、database 必须全指向本次测试根，不能只改变 config 文件位置。

| cwd | 命令 | 成功与局限 |
|---|---|---|
| DIVA | `pnpm --dir agent-diva-gui install --frozen-lockfile` | 按现有 pnpm 锁安装 |
| DIVA | `python3 scripts/build-desktop.py --mode test --vivy-dir /absolute/pinned-vivy --laputa-dir /absolute/pinned-laputa` | 支持的 consumer.mod 构建/race gate；不能用未解析本地 replace 的裸 go test 替代 |
| DIVA | `python3 scripts/build-desktop.py --mode build --vivy-dir /absolute/pinned-vivy --laputa-dir /absolute/pinned-laputa --output /absolute/work/memory-candidate` | 密封构建/Inspect；记录锁漂移，候选 repin 仅在执行分支 |
| DIVA | `pnpm --dir agent-diva-gui exec vitest run src/api/cognitive.test.ts src/state/vivy-cognitive.test.ts src/components/persona-memory/PersonaMemoryView.test.ts` | 既有认知 UI gate |
| VIVY | `go test ./internal/runtime ./internal/observerhost ./internal/modules/diva-cognitive ./internal/app -count=1` | 现有运行时回归，不等于真实 DIVA 产品通过 |
| Laputa/garden | `go test ./agentapi ./internal/runtimecore ./internal/recall ./internal/ingest ./memory/... ./evolution ./backends/mentle -count=1` | 领域及 adapter 回归 |
| Laputa/laputa | `go test ./persona ./actmem ./evolution/... -count=1` | 人格、活动、策略 |
| Laputa/mentle | `go test ./facade ./storage/sqlite -count=1` | canonical 和持久回执 |
| Laputa/garden/console | `pnpm install --frozen-lockfile`，随后 `pnpm build` | 此目录已有 pnpm-lock.yaml，无 package-lock.json；先生成 embed 资源 |
| Laputa/garden | `go test -tags=e2e ./e2e/... -count=1` | 区分 fake 与 real backend 的证据范围 |

新增测试按各 Story 的精确 `-run` 执行。先用 `go test <package> -list '<regex>'` 确认名称存在，再跑 `-json -count=1` 并记录真实数量；零匹配和 required skipped 不能通过。测试资源缺失时返回明确 blocked，不因跳过而绿色。

最终改动运行受影响仓库规定的完整门槛：VIVY `just ci`，Laputa 三模块全套及 Garden e2e，DIVA `pnpm --dir agent-diva-gui test` / `build` 和密封 Go test/build。DIVA 当前 `just ci`有旧 Rust bridge，未覆盖全部 Wails 记忆验收；不能单独作为放行证据。CGO=0 的可移植回归不代替实际本地模型/Windows 正向测试。

Windows 从候选产物启动，使用 `diva.exe --config <绝对测试配置路径>` 或 `DIVA_VIVY_CONFIG`。确认 GUI 加载、原生进程、配置根与 artifact 相符，再执行 S12。需要预建模型目录时走支持的安装/配置流程并记录，不从用户 profile 拷贝已存好的记忆。

## 3. 共享文件和执行顺序

| 共享范围 | 主责 Story | 修改协调 |
|---|---|---|
| build/vivy-sources.lock.json 与候选配置 | S01；后续修复返回 S01 记录新 candidate | 不并行 repin |
| memory_loop_fixture_test.go / checker | S03；S10 只增加 Crash 扩展 | 先交付接口，再追加消费者；扩展须同步 contracts |
| cognitive_service.go / capture | S04，随后 S06/S10 | 依赖内顺序修改，不新建重复 capture |
| Garden runtime/backend | S02，随后 S11 | S11 若改共享文件，先合入 S02 基线 |
| ACTMEM / persona authority | S05 / S07 | 不能将 persona review 与普通记忆效果合并 |
| context / recall | S08，随后 S09 | 先证明原始召回，再验证纠正删除 |
| README 状态 / package-map 镜像 | 集成负责人 | 单写入人，验证表格一致 |

W2 的 S02/S03、W9 的 S10/S11 可在不同文件和环境上独立准备；默认由同一负责人串行实现，共享改动在批次汇合时统一验证。不把并行机会写成无资源依据的工期压缩承诺。

## 4. 每个 Story 的交接清单

- [ ] 直接前置的结果可读且 candidate 匹配。
- [ ] 所有新增测试实际被执行，原始失败/修复/通过证据保留。
- [ ] 真实存储、脚本模型、Windows UI、真实模型证据模式标明。
- [ ] summary.md 给出问题、结果、范围和下一个可开始的 Story。
- [ ] verification.md 给出命令、退出码、数量、环境与原始 artifact 路径。
- [ ] evidence.json 经 checker 验证；缺陷记录入根 TODOLIST，附 Story/case 与复现。
- [ ] 各仓库只提交本 Story 的变更，保留人类作者身份；未授权不 push。
- [ ] README 状态含实际 commit 和证据链接；释放 LOCK。

## 5. 阻塞处理与重新排期

Story 发现缺口时先交付可复现证据，不把发现问题当作失败的工作。Blocked 记录负责角色、解阻动作、预计新增人日与受影响后继。已复现缺陷满足 [修复准入](continuity.md)时可以继续定位与最小修复；验收仍按原前置执行，不能越过 gate 宣称后继通过。

修复预留统一为 3–5 人日，不在每个 Epic 重复加一份。新架构、后台替换、模型分发机制或重大恢复协议变更超出该预留，需单独设计和重估。S01/S02 完成后第一次重估，发现召回路径缺失后再次重估。

进度按 Epic/Story 的 Done/Blocked 及可用证据汇报，不再按日历时段拆任务。等待模型资源、Windows、凭据和联网环境单独记录。

## 6. 最终报告骨架

1. 验收 candidate 与各平台 artifact。
2. 28 类场景及所有必需重复样本的 pass/fail/blocked。
3. 对话→来源→canonical→reflection→recall→prompt→回答的三份完整链路。
4. 六切点 × 十次恢复与零泄漏/零重复结论。
5. 三十次真实模型原始样本、固定 rubric、正确数、耗时与 token。
6. 未完成能力、首个断点、最小修复建议、适用环境。

所有必需场景未通过时，结论为部分成立或不成立；计划包完成不等于记忆闭环成立，也不等于 DIVA W7 发布签收。

## 7. 本轮可复现的集成准备命令

实际工作根为 `/workspace/work/memory-loop`，三个仓库均为隔离 worktree。设置 task-local Go 编译器（系统 `go` 是 GNU Go 棋类程序）：

```sh
export PATH=/workspace/work/memory-loop/tools/go/bin:$PATH
export GOMODCACHE=/workspace/work/memory-loop/tools/gomod
export GOCACHE=/workspace/work/memory-loop/tools/gocache
```

在 VIVY 根执行实际 SDK pack 和受检 overlay 生成（更改 internal 输入后必须重新 pack，旧 manifest 会被拒绝）：

```sh
GOFLAGS=-buildvcs=false go run ./sdk pack --target shared --recipe recipes/diva.vivy.yml --output /workspace/work/memory-loop/artifacts/diva-kernel-reviewed
VIVY_MEMORY_LOOP_ARTIFACT=/workspace/work/memory-loop/artifacts/diva-kernel-reviewed VIVY_MEMORY_LOOP_OVERLAY_DIR=/workspace/work/memory-loop/tools/diva-overlay go test -json -tags vivy_headless ./sdk/internal -run '^TestMemoryLoop(GenerateIntegrationOverlay|OverlayRejectsMismatchedInputs)$' -count=1
TMPDIR=/workspace/work/memory-loop/test-roots VIVY_MEMORY_LOOP_EVIDENCE_DIR=/workspace/work/memory-loop/logs/fixture-artifacts go test -json -tags 'vivy_headless vivy_diva_integration' -overlay /workspace/work/memory-loop/tools/diva-overlay/overlay.json ./internal/app -run '^TestMemoryLoopFixtureUsesRealComposition$' -count=1
```

测试目录先创建。当前托管环境中 loopback HTTP 也需要工具的 network 权限。`vivy_diva_integration` 只排除默认 recipe 专属 inventory 测试；默认 App 回归另跑。共享库用途是检验真实 recipe composition，不代替 Wails host、Windows 原生 UI 或真实模型验收。

Garden Console 的 pnpm shim 限制和本轮脚本正文 fallback 见 environment.md。`just`/PowerShell 不可用，VIVY justfile 允许直接 Go 命令；本轮只跑了记录的受影响回归，未声称完整 CI。原始失败日志和每条命令退出码不可删除或改写为通过。

## 8. 可持续修复的选择规则

每轮先查 README 的开发序列和 continuity.md。环境/平台出口为 Blocked 时，选择有真实输入和失败证据的最高优先级代码修复；不要把所有修复都挂在 sealed host 的出口。保留原始失败日志，重新构建开发 candidate 并跑相应回归；所有原始验收门槛继续保留。
