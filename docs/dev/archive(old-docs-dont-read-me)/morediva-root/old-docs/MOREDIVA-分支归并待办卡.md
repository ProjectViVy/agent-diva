# MOREDIVA 分支归并待办卡

更新日期：2026-06-06  
工作目录：`C:\Users\Administrator\Desktop\morediva`  
适用范围：`agent-diva`、`agent-diva-pro`、`agent-diva-selfinprove`、`agent-diva-agent-new`、`agent-diva-sandbox`、`agent-diva-channel-adapter-and-plugins`、`agent-diva-tui`  
补充说明：`agent-diva-vrm-memory-test` 本轮 live 扫描未发现本地仓路径，暂列为可选泳道。  
上游参考：`MOREDIVA-分支归并决策.md`、`agent-diva/docs/dev/main-closeout-plan-2026-06.md`、`agent-diva/docs/dev/main-closeout-cards-2026-06.md`

## 0. 总规则

- 本轮不是“整支 merge”，而是“按主题分流归并”。
- `agent-diva/main` 只收 backend / runtime / safety / infrastructure。
- `agent-diva-pro` 只收 product / GUI / workflow / user-visible capability。
- `agent-diva-selfinprove` 固定为研究母线，不再承担长期产品交付职责。
- `agent-diva-agent-new` 视为与 `selfinprove` 高重叠的孵化副本，先去重再决定归档。
- `sandbox`、`channel-adapter-and-plugins`、`tui` 统一按“历史线”处理：先抽干价值，再冻结归档。
- 在治理完成前，不再往研究线/历史线继续落新功能。

## 1. Live 角色快照（2026-06-06）

| repo | 当前分支 | 角色 | 下一动作 |
|---|---|---|---|
| `agent-diva` | `main` | 稳定后端主线 | 执行 repo 内 `MAIN-CLOSE-*` 收口 |
| `agent-diva-pro` | `feature/context-compaction` | 产品实现线 | 清理脏树，拆成产品主题卡 |
| `agent-diva-selfinprove` | `autoresearch/...` | 研究母线 | 建 authoritative index，冻结角色 |
| `agent-diva-agent-new` | `agent-diva-agent-new` | 重叠孵化副本 | 对比 `selfinprove`，提取独特点 |
| `agent-diva-sandbox` | `agent-diva-with-sandbox` | 历史实验线 | 提取残值后归档 |
| `agent-diva-channel-adapter-and-plugins` | `agent-diva-plugings-and-adapter` | 历史文档壳 | 归档/零星提取 |
| `agent-diva-tui` | `main` | 历史文档壳 | 归档/零星提取 |
| `agent-diva-vrm-memory-test` | 未发现本地路径 | 可选专项线 | 路径恢复后单独评估 |

## 2. 卡片总览

| ID | target | 优先级 | 依赖 | 目标 |
|---|---|---|---|---|
| `GOV-01` | `morediva/docs` | P0 | 无 | 冻结 canonical 路由规则 |
| `RESEARCH-01` | `agent-diva-selfinprove/docs` | P0 | 无 | 固定研究母线与 authoritative docs |
| `MAIN-01` | `agent-diva` | P0 | `GOV-01` | 完成 `main` backend-only closeout |
| `MAIN-02` | `agent-diva` | P0 | `RESEARCH-01` | 从研究线抽 backend contract 到 `main` |
| `PRO-01` | `agent-diva-pro` | P0 | `GOV-01` | 规范 `pro` 脏树，只保留产品落点 |
| `PRO-02` | `agent-diva-pro` | P1 | `RESEARCH-01`, `PRO-01` | 把 Journal/Inbox/Review/Self-evolution 主题落到 `pro` |
| `PRO-03` | `agent-diva-pro` | P1 | `MAIN-02`, `PRO-01` | 产品化 rhythm / compaction status 与设置面 |
| `RESEARCH-02` | `agent-diva-agent-new/docs` | P1 | `RESEARCH-01` | 去重 `agent-new`，提取独特残值 |
| `ARCHIVE-01` | `agent-diva-sandbox/docs` | P1 | `GOV-01` | 提取 sandbox 残值并冻结 |
| `ARCHIVE-02` | `agent-diva-channel-adapter-and-plugins/docs` | P2 | `GOV-01` | 提取残值并归档 |
| `ARCHIVE-03` | `agent-diva-tui/docs` | P2 | `GOV-01` | 提取残值并归档 |
| `OPTIONAL-VRM-01` | `vrm/docs` | P3 | 本地路径恢复 | 仅在仓恢复后再开专项泳道 |

## 3. 卡片详情

### `GOV-01` Canonical 路由冻结
- **目标**：把“main/backend、pro/product、selfinprove/research、其余/archive”的路由规则固定成后续唯一准绳。
- **来源**：本文件 + `MOREDIVA-分支归并决策.md` + live git 快照。
- **验收**：后续所有迁移卡都显式引用目标 landing zone；研究线/历史线明确标注“禁止继续承接新功能”。
- **非目标**：不移动代码，不处理具体实现冲突。

### `RESEARCH-01` selfinprove 母线冻结
- **目标**：把 `selfinprove` 从“并行实现线”正式降级为“研究母线”。
- **来源**：`docs/dev/genericagent/*`、`docs/logs/2026-05-*`。
- **验收**：有 authoritative docs 索引；有 superseded/archive 标记；新人不必重读全量日志才能知道当前母本。
- **非目标**：不在该卡里做代码实现或整支 merge。

### `MAIN-01` main closeout
- **目标**：按 repo 内现成的 `MAIN-CLOSE-01..05` 完成 `main` 收口，恢复 backend-only 身份。
- **来源**：`agent-diva/docs/dev/main-closeout-plan-2026-06.md`、`main-closeout-cards-2026-06.md`、`TODOLIST.md`。
- **验收**：`main` 工作树不再混入 product/UI 面；被排除的前端改动有明确 moved-out / reroute 结论。
- **非目标**：不在 `main` 内落 Journal/Inbox/Review/Self-evolution UI。

### `MAIN-02` 抽 backend contract 回主线
- **目标**：从 `selfinprove` 中只抽真正属于后端的主题进入 `main`。
- **来源**：`shared-memory-rendering-research.md`、`candidate-evidence-journal-audit-design.md`、`autodream-rhythm-distillation-design.md`、`context-compaction-research.md`。
- **范围**：Shared MEMORY 渲染 MVP、Candidate/Evidence/Audit 最小 schema、AutoDream trigger/lock/checkpoint skeleton、context budget/runtime guardrail。
- **验收**：形成 file-level extraction shortlist；落点无 GUI 依赖；不把研究日志整包带进主线。
- **非目标**：不做 Review UI / Self-evolution UI。

### `PRO-01` 规范 pro 当前脏树
- **目标**：把 `feature/context-compaction` 当前脏树拆成“留在 pro / 回流 main / 丢弃或归档”三类。
- **来源**：当前 `agent-diva-pro` 工作树、`docs/adr/0010-context-compaction.md`、`docs/prds/prd-agent-diva-pro-2026-06-03/*`。
- **验收**：每个变更文件都有归属；避免形成又一个混合大提交；`pro` 保持产品落点身份。
- **非目标**：不直接完成所有功能开发。

### `PRO-02` 落产品工作流主题到 pro
- **目标**：把 `selfinprove` 中用户可见的产品面明确落到 `pro`。
- **来源**：`newedge/ui-design.md`、`agent-diva-pro-self-evolution-ui-research.md`。
- **范围**：Journal、Inbox、ReviewCard、EvolutionProposalCard、EvidencePeekCard、Self-evolution 入口与导航关系。
- **验收**：拆成可执行产品卡/epic；每张卡只负责一个产品主题；落点全部在 `pro`。
- **非目标**：不改后端协议层归属。

### `PRO-03` 产品化 rhythm / compaction 可视面
- **目标**：在 `pro` 上承接 rhythm / compaction 的状态可视化、设置入口、审批入口。
- **来源**：`autodream-rhythm-distillation-design.md`、`context-compaction-research.md`、当前 `pro` 分支。
- **验收**：至少拆出 status panel、settings、candidate review、error visibility 四类产品卡；依赖 `MAIN-02` 的 backend skeleton。
- **非目标**：不在第一版里做全套调度策略或完整历史查询。

### `RESEARCH-02` 去重 agent-new
- **目标**：把 `agent-new` 与 `selfinprove` 的重叠研究去重，只保留真正独特的残值。
- **来源**：两边 `docs/dev/genericagent/*` 与相关 `docs/logs/*`。
- **验收**：产出 unique-only 清单；重复材料标注 superseded/archive；`agent-new` 不再被当活跃实现线。
- **非目标**：不做整支 merge 到 `selfinprove` 或 `pro`。

### `ARCHIVE-01` sandbox 残值提取与冻结
- **目标**：确认 `sandbox` 还剩什么值得要，剩余全部归档。
- **来源**：当前 `sandbox` docs/dev 残留、历史整合记录、已吸收入 `pro` 的能力。
- **验收**：形成 keep-list / archive-list 二分表；之后不再把 `sandbox` 当主功能线。
- **非目标**：不重启 sandbox 作为长期实现仓。

### `ARCHIVE-02` channel-adapter-and-plugins 归档
- **目标**：把该仓剩余价值压缩成最小 extraction note，然后冻结。
- **来源**：当前 `docs/dev` 差异。
- **验收**：要么确认“无剩余工程价值，仅归档”，要么产出极短提取清单指向 `main` / `pro` / docs。
- **非目标**：不新开 adapter/plugin 功能开发。

### `ARCHIVE-03` tui 归档
- **目标**：把 `agent-diva-tui` 也按历史线处理，不再保留为模糊活分支。
- **来源**：当前 `docs/dev` 差异。
- **验收**：产出 archive note；若有残值，仅输出短 extraction note。
- **非目标**：不重启一条新的 TUI 产品线。

### `OPTIONAL-VRM-01` VRM 专项泳道（仅路径恢复后）
- **目标**：把 VRM 从本轮 morediva 收口中剥离，单独评估是否值得抽取。
- **来源**：仅在仓路径恢复后再读 live 状态；本轮不依赖旧记忆直接开卡。
- **验收**：确认本地仓存在、分支状态可读、范围独立后，才允许创建后续 VRM 抽取卡。
- **非目标**：不基于过期印象做 speculative merge 计划。

## 4. 建议执行顺序

1. `GOV-01`
2. `RESEARCH-01`
3. `MAIN-01`
4. `MAIN-02`
5. `PRO-01`
6. `PRO-02`
7. `PRO-03`
8. `RESEARCH-02`
9. `ARCHIVE-01`
10. `ARCHIVE-02`
11. `ARCHIVE-03`
12. `OPTIONAL-VRM-01`（仅仓路径恢复后）

## 5. 一句话收口

先把 `main` 收成干净 backend 主线，再把 `pro` 固定成产品落地区；`selfinprove` 做研究母线，`agent-new` 去重降级，`sandbox / channel-adapter-and-plugins / tui` 抽干后归档。整个过程按主题迁移，不做整支 merge。
