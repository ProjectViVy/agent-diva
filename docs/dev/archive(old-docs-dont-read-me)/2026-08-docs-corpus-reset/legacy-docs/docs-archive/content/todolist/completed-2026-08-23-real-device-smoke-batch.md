# 完成归档：2026-08-23 真机冒烟批次与 Clean Break EPIC 收口

2026-08-23 用户确认：积压的真机桌面冒烟批次全部通过，冒烟期间修复的一批小 BUG
已上主线（`dev` / `main`）。同时归档根清单中此前已勾选、仅作完成物指针的历史项。
归档规则同 [`README.md`](README.md)：保留原始上下文，不再驱动实施或验收；恢复须先
重新验证问题仍存在，再以新活跃条目写回根清单。

## 一、EPIC 收口：Laputa 认知工作区 Clean Break（2026-08-23 关闭）

- [x] **LAPUTA-COGNITIVE-WORKSPACE-RESET**（总 EPIC，`sev-P0`）
  D0–D4 批准、保护分支 `protect/cognitive-pre-clean-break-20260815` @ `2aab18cc`
  （本地，未 push，不作 fallback）、S1–S5 实施、S6 机器证明门
  （`just cognitive-clean-break-check` 已进 `just ci`）与真机桌面冒烟全部通过。
  编排：`docs/research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md`；
  架构：`docs/architecture/laputa/architecture.md`。
- [x] **COGNITIVE-I1-CLEAN-BREAK-IMPLEMENTATION** `sev-P0`
  S1 停种子 → S2 Persona（`93f4b554`）→ S3 Memory/ACTMEM/Recap → S4 Skill →
  S5 卸旧（`710e7684`..`1ea54d08`，复核修复 `0c1f0dfd`..`c011e680`）→
  S6 证明（`2e58a022` 机器门 + 2026-08-23 真机八条与恢复演练通过）。
- [x] **UI-S2-PERSONA** / [x] **PERSONA-S2-DESKTOP-SMOKE**：Persona 文档工作区
  与真实桌面验收（五文件引导、incomplete 修复、CM6 编辑/预览、CAS 冲突、
  pending 接受/拒绝、历史 Diff；2026-08-17 曾修首启 `unknown Laputa API error`）。
- [x] **UI-S3-MEMORY-ACTMEM** / [x] **MEMORY-S3-DESKTOP-SMOKE**：BML 直改无审批、
  ACTMEM Pulse/Recap/Work/胶囊、MEMRULES 可编，真机验收通过。
- [x] **UI-S4-EVOLUTION-SKILL** / [x] **EVOLUTION-S4-DESKTOP-SMOKE**：Skill 列表/
  编辑/CAS/历史/待审与 Evolution 无混箱，真机验收通过。
- [x] **ACTMEM-S3-RECAP** `sev-P1`：每轮助手回复结束立刻写 Recap（≤200 字、
  不另开大模型），Pulse 只收用户短原话，10 分钟空闲仅折叠胶囊。
- [x] **WORLD-MEMRULES-GATE** `sev-P2`：R6 进 WORLD 写核（`352cd57f`），
  提案必须是有界 claim，AutoDream 不能改既有 claim。
- [x] **MEMRULES-DEFAULT-SEED-ALIGNMENT** `sev-P3`：默认 R1–R7 随 S3 落地，
  首次用户保存才创建 `{config_dir}/memory/MEMRULES.MD`。

EPIC 冻结产品边界（Memory/Persona/BML/Evolution/MEMRULES/Approval 决策）不再随
根清单维护，权威依据固化于：
`docs/research/cognitive-workspace-reset-epic-2026-08/`、
`docs/research/evolution-genericagent-reset-2026-08/decision-record.md`、
`docs/research/persona-markdown-clean-break-2026-08/decision-record.md`、
`docs/research/stm-cross-session-clean-break-2026-08/decision-record.md`。

## 二、Research / Architecture / Implementation Gate 完成物指针

- [x] **COGNITIVE-R0-CURRENT-STATE**：`docs/research/cognitive-r0-current-state-2026-08/README.md`
- [x] **COGNITIVE-R1-GENERICAGENT-EVOLUTION**：`docs/research/cognitive-r1-genericagent-evolution-2026-08/README.md`（GA 锁定 `ee5a474`；进化改走 D6）
- [x] **COGNITIVE-R1c-GA-AUTONOMY-ORIGIN**：`docs/research/cognitive-r1-genericagent-evolution-2026-08/ga-autonomy-origin.md`
- [x] **COGNITIVE-R2-STM-CONTEXT**：`docs/research/cognitive-r2-stm-context-2026-08/README.md`（分层旧提案部分作废）
- [x] **COGNITIVE-R3-PERSONA-WORKSPACE**：`docs/research/cognitive-r3-persona-workspace-2026-08/README.md`
- [x] **COGNITIVE-R4-CLEAN-BREAK-SAFETY**：`docs/research/cognitive-r4-clean-break-safety-2026-08/README.md`
- [x] **COGNITIVE-D0-DOMAIN-AUTHORITY** / **D1-PERSONA** / **D2-MEMORY-STM** /
  **D3-EVOLUTION-SKILL** / **D4-CLEAN-BREAK-DELIVERY**：用户 2026-08-15 批准，
  正文见 `docs/research/cognitive-d{0..4}-*-2026-08/`。
- [x] **COGNITIVE-I0-PROTECTION-BASELINE**：2026-08-15 已切
  `protect/cognitive-pre-clean-break-20260815` = `2aab18cc…`（本地、未 push、非 fallback）。

## 三、真机桌面冒烟批次（2026-08-23 用户确认通过，修复已上主线）

- [x] **CHANNELS-RETIRE-DESKTOP-SMOKE** `sev-P2`：退役频道 GUI 下架（卡片 7 张、
  向导与侧边栏不含退役频道）+ 后端 opt-in `channel-*` feature 门控。
- [x] **CHANNELS-EDITOR-DESKTOP-SMOKE** `sev-P2`：卡片编辑空白/布局/YAML-only 修复后
  的飞书/Email 编辑、保存刷新、Discord allow_from、全通道表单真机点选。
- [x] **QQ-CHANNEL-DESKTOP-SMOKE** `sev-P2`：空 `allow_from` 不限制 + QQ 群/频道事件
  拒绝不进 bus，C2C 私聊与群 @ 拒绝真机验证。
- [x] **SKILL-MARKETPLACE-DESKTOP-SMOKE** `sev-P2`：skills.sh 市场搜索/安装/精选榜单
  （v0.2.0）/已安装删除（v0.2.1）/Evolution 视图过滤（v0.2.2）真机验证。
- [x] **M3-HITL-DESKTOP-SMOKE** `sev-P2`：谨慎/智能/信任三模式、trusted 规则学习、
  重启保持与 Guardian 限制集中验收（`docs/logs/2026-08-m3-hitl-closure/`）。
- [x] **SANDBOX-SAVE-FIX-DESKTOP-SMOKE** `sev-P2`：沙箱模式切换保存、配置落盘与
  清空 timeout 边界。
- [x] **CLARIFY-HITL-DESKTOP-SMOKE** `sev-P2`：真实 LLM 触发 `ask_user`，CLI/GUI
  提问、回答、超时与恢复。
- [x] **GATEWAY-PORT-DESKTOP-SMOKE** `sev-P3`：`gateway.port` 配置生效，未配置仍 3000。
- [x] **GUI-PROVIDER-ERROR-RETRY-DESKTOP-SMOKE** `sev-P2`：失败/重试/stall/最终错误
  展示，不因 request ID 或 SSE 断流永久挂起。
- [x] **MEMORY-CRUD-ID-DESKTOP-SMOKE** `sev-P2`：运行路径 PII 隐藏关闭后，真实模型
  `memory_list` → 新增 → `memory_update`/`memory_remove`，id 与邮箱/手机原文可见
  （`docs/logs/2026-08-memory-id-pii-redaction/v0.0.2-disable-runtime-pii-hiding/`）。
- [x] **WINDOWS-RELEASE-EXEC-ACCESS** `sev-P1`：真机冒烟批次在 release 可执行上通过，
  原 OS error 5 启动阻断随本批次解除；后续如复发须重新开条目。

## 四、此前已勾选的历史完成项（自根清单移入）

- [x] **GOVERNANCE-LEDGER-SINGLE-RECORD-BRICK** `sev-P2`：2026-08-17 修复重复
  `expired` 重放幂等、`expire()` 不二次落盘、孤立聚合跳过告警
  （`docs/logs/2026-08-governance-ledger-startup/v0.0.1-expire-idempotent-recovery/`）。
- [x] **EMPTY-POST-TOOL-SUMMARY** `sev-P1`：空 follow-up 按输入压力/输出截断/空 stop
  分类，最多一次 summary-only（禁工具、8192 输出预算）
  （`docs/logs/2026-08-empty-tool-summary/v0.0.1-upstream-empty-followup/`）。

## 五、2026-08-23 二次清理（被取代 / 失效项，用户确认直接清理）

- [~] **COGNITIVE-R1b-GA-LIVE-EXPERIMENT** `sev-P2`（被取代）：GenericAgent 活体结晶
  实验属旧 GA 参考路线；用户已决策进化走 D6、不再跟 GA，EPIC 亦已关闭，实验失去
  服务对象。原描述：用桌面 `keys.txt` 测 Action-Verified 遵守率、未验证写入率、
  L1 行数违规与 patch/overwrite 比。
- [~] **CONTEXT-DENSITY-PRINCIPLE** `sev-P1`（转为研究参考）：条目自述「现在不施工」，
  非可执行待办；内容已固化于
  `docs/research/cognitive-r1-genericagent-evolution-2026-08/ga-context-density-vs-diva.md`，
  后续 D0/C 系设计时直接对照该笔记即可，不再占用活跃 backlog。
- [~] **UX-DR-3/4/7** `sev-P3`（关闭，复发再重开）：Sprint 评审遗留 UX 缺口；
  条目自设前提「恢复前先重新确认原问题仍存在」。2026-08-23 真机冒烟全过、
  功能基本正常，未见复现；按用户决策关闭，若后续复现须重新开条目并补专项设计。
