# Laputa PRD — Adversarial Review (会失败点)

> 角色: Reviewer Gate 的 adversarial 子 agent, 跟 rubric walker 互补 — walker 评 shape / substance, 本文件专找 "PRD 哪里会失败"。
> 材料: prd.md v0.0.4 (323 行) + .decision-log.md (D-001~D-009) + 3 份 reconcile + 已有 review-rubric.md。
> 立场: 不重写 PRD, 不挑战 D-001~D-009 已落档决策, 只标 "哪里执行时会塌"。

---

## 总体判定: BLOCK-pending-patch

PRD 下半 (§5 FR 层) 写得密 — 32 条 FR 多数有 grep / 数字 / 命名约束级验收, 但本 review 发现 **3 个真 critical (会直接打住实现) + 5 个 high (实现时会反向咬人)**, 加上 6+ 个 medium 必须 polish 阶段处理。1 critical 走 §6 OQ #4 升 P0 + FR-501 写权清单闭合; 1 critical 走 FR-102/502/503 一致性修; 1 critical 走 selfinprove 集成 FR 补 (D-011 锁的 stub 实接无对应 FR)。建议: 进入 polish 阶段前先修 3 critical, 否则 launch review gate 站不住。

---

## CRITICAL (3 条 — 不修就崩)

### C-1. TBD 14 section 写权契约整张表是空的 (PR §1 Vision 答不了 / §5 FR-501 答不了)

- §4.1 (prd.md L77-81) 把 14 section 拆 "Laputa-owned 6 / Report-owned 3 / TBD 5", 后续 FR-501 (L196-203) 列 schema 字段 5 个全含枚举值, 但 **14 行里 `write_authority` 没填一格**。
- §6 OQ #4 (L290) "SOUL.md AI 自写用户怎么干预" 仍 **开**, D-008 把它列入 P1 (L308 "本 PRD 补"), reconcile-mentle-laputa.md actionable #5 + reconcile-auto-evolution.md actionable #8.① 都升级 P0, 但 PRD 没合。
- 后果链:
  - 工程师无法实现 FR-101 (L99) — 不知道 selfinprove 写时哪些 section 走 apply_proposal 哪些走 skip;
  - FR-702 (L263-264) 审计 grep 也查不出"未授权写", 因为"授权"未定义;
  - §1 Vision (L39-43) "Laputa 跟现有系统边界" 写"selfinprove 第 1 写消费方"等 3 行是方向性, 缺"具体到 #1/#3/#4 的写权清单" — Launch review 上 PM 答不出 "用户怎么改 agent 身份"。
- 修法: §5.5 FR-501 补 1 条子要求 (类似"#1 identity / #3 commitment = shared; #4 preferences = user_only; #5 memory_md / #6 history_md = agent_self"), §6 OQ #4 升 P0 + 写 "兑现 PR"。

### C-2. TASK.md / #10 journal_reflective 迁移路径自相矛盾 (PR §4 vs §5 vs §5 自相矛盾)

- FR-502 (L215) 写: "TASK.md → #10 journal_reflective ⚠️"
- FR-102 (L112-113) 写: "MUST 拒收 proposal_type 未知或**目标 section TBD** (#10-14 中 4 项待最终综合 PRD) 的 proposal, 返回 Err::UnknownLayer"
- FR-503 (L219-224) step 3 写: "存在 → 增量迁移: 旧 8 文件如有内容, **按 FR-502 映射迁到 1 稀薄层**"
- 后果链 (runtime will fail):
  - 旧装用户升级时, workspace 有 TASK.md, 里面可能含 100+ 条任务;
  - 启动走 `sync_workspace_templates()` 改写后, 按 FR-503 step 3 调 apply_proposal 把 100 条按 proposal_type 写;
  - FR-102 拒收 #10 journal_reflective (TBD) → **整次 migration 返 Err::UnknownLayer**;
  - 旧文件备份到 `.laputa/legacy/{ISO8601}/` 但内容未迁入新层 → **用户数据丢失**;
  - 验收 "旧装升级, 旧 8 文件内容迁到 1 稀薄层" (L224) 跑不过。
- 附带矛盾: FR-102 L113 写 "**4 项** TBD", §4.1 L81 + FR-205 L155 写 "**5 项** TBD" — 数字都不一致, 工程师实现校验时会卡死 (取哪个?)。
- 修法: 二选一 (a) FR-502 把 TASK.md 标 "启动期暂存" 不入 14 section, FR-503 step 3 加 "TASK.md 跳过 / 备份 + 不迁"; (b) FR-102 把 #10 从 TBD 排除, 标 "TASK.md 迁移期临时接收"。**D-005 L122 "TBD 走通用 section 写入接口" 暗示 (b), 但 FR-102 反向拒收 — 决策没合。** 同时把 "4 项 / 5 项" 数字合到 5。

### C-3. selfinprove stub 实接无对应 FR (D-011 锁的工作没人接)

- D-011 (prd.md L16) 锁 "一锅端 ship, selfinprove FR-5xx 走 stub, **待本 PRD 实接**"
- D-002 L39-43 列 5 个 hook 点, 其中 3 个**无任何 FR 引用**:
  - `agent-diva-manager/src/runtime.rs:335-365` 写权限实接入口 (L39)
  - `agent-diva-core/src/memory/provider.rs:247-275` `laputa_wakeup` / `laputa_project_soul` / `laputa_recall_intent` 3 命名边界 (L42)
  - `agent-diva-core/src/memory/provider.rs:125-139` `StartupContextSnapshot.laputa_state_root` 已存在的启动点 (L40)
- PRD 32 条 FR 只引用了 2 个 hook: FR-503 引 `mod.rs:91-105` `sync_workspace_templates()`, FR-601 引 `agent_loop.rs:570 mentle_active()`。**runtime.rs:335-365 / provider.rs:247-275 / provider.rs:125-139 这 3 个真正的 integration point 无 FR**。
- 后果链 (ship will fail):
  - 工程师实现完 FR-1xx~7xx 后, 跑起来 Laputa 是孤儿 crate, 没人调;
  - selfinprove FR-5xx stub 还是 stub, 因为没人写 "把 stub 替换成 `crate::laputa::apply_proposal`" 的实接代码;
  - D-008 锁的"完成后单独发 Sprint Change Proposal 拆 stub"前提是 stub 真拆了, 现在 PRD 不规定拆法, 等于 Sprint Change Proposal 写不出来;
  - reconcile-auto-evolution.md actionable #1 + #3 + §3 同源指出, 这是 §5 缺 "End-to-End P0 Loop" + "MemoryProvider Lifecycle Hooks" 两条 FR 段的根因。
- 修法: §5.1 增 1 段 "Integration Hooks" FR, 至少 3 条: (a) FR-110 `runtime.rs:335-365` 必须调 `LaputaWrite::apply_proposal` (替换 selfinprove FR-5xx stub); (b) FR-111 `provider.rs:125-139` StartupContextSnapshot 必须读 Laputa FR-2xx 替代直读 state.json; (c) FR-112 `provider.rs:247-275` 3 命名边界 (laputa_wakeup / project_soul / recall_intent) 走 LaputaWrite / LaputaRead 入口而非直 fs 调。

---

## HIGH (5 条 — 实现时会反向咬人)

### H-1. FR-103 4 道治理 atomic 在 audit 步不可达成

- FR-103 (L115-122) 写 "4 道 MUST 串联, 任一失败 MUST 整体回滚 (atomic)", 验收 L122 "模拟 1 道失败, 整体回滚"
- 4 道顺序隐含 (L116-120): review → changelog → write → audit
- **bug**: write 已 commit, audit 才发。如果 audit 失败 (e.g. AuditEvent 写文件 IO 错, 或发外部 sink 网络错), write 已发生, 不可回滚 — 因为 review/changelog/write 都已 OK, 用户视角看 state.json 已变。**回滚 = 再次写 changelog, 不是 atomic**。
- 后果: FR-705 L275 列 7 种错误类型含 "IoError", 但 IoError 在 audit 步是 "写完才能报", 写完时报的 Err 不是 "写失败", 是 "审计失败" — PRD 没区分这 2 种 Err 语义。
- 修法: FR-103 把 4 道改 "review → changelog → audit-tentative → write → audit-commit" 2 阶段; 或显式说 "audit 失败不回滚, 写 needs_attention AuditEvent 即可", 验收改 "audit 失败不阻塞写, 必须 needs_attention 通知"。

### H-2. FR-702 grep 验收可绕过

- FR-702 (L263-264): "Code review MUST 检查: 任何调 `fs::write(.*\.laputa` 路径的代码必须走 `LaputaWrite::apply_proposal`"
- 验收 (L264): "grep 整个 agent-diva-pro 仓库, `fs::write(.*\.laputa` 命中 0 处 (除 LaputaWrite 实现内)"
- **可绕过** (rust 生态常见):
  - `tokio::fs::write(path, bytes).await` — grep `fs::write` 漏;
  - `std::fs::File::create(path)?.write_all(bytes)?` — 完全逃逸;
  - `OpenOptions::new().write(true).create(true).open(path)?` — 逃逸;
  - `serde_json::to_writer(BufWriter::new(File::create(path)?), &state)?` — 逃逸;
  - helper 函数 `pub fn save(path: &Path, data: &[u8]) { fs::write(path, data) }` — 内部藏一层 grep 也漏;
  - shell out 到 `std::process::Command::new("sh").arg("-c").arg("echo x > .laputa/state.json")` — 完全逃逸。
- 后果: 验收 "grep 0 命中" 实现后, 下个 sprint 别人加 1 个 `tokio::fs::write` 绕过, 审计失效, FR-103 review/changelog 全部失效。
- 修法: FR-702 验收改 "lint rule (e.g. cargo-deny / custom clippy lint) 禁止 .laputa 路径下任何 write 形式", 不可靠纯 grep。

### H-3. FR-301 60s 轮询 fallback 端点不存在

- FR-301 (L161) 写: "MUST 支持 fallback: 客户端 MUST 能用 `GET /api/laputa/proposals?since=ISO8601` 60s 轮询, 功能等价 SSE"
- 验收 (L163): "SSE 断, 切轮询, 60s 内拉到同一 proposal"
- **bug**: PRD FR-2xx 读接口 (L138-149) 列出 5 条: snapshot / section / since / changelog / tbd 标志, **没有 `/api/laputa/proposals` 端点**。FR-202 (L144) 列 14 section name 不含 "proposals"。
- 后果: 客户端开发按 FR-301 实现 fallback, 发现端点 404 / 405。launch 后只能临时补 1 条 FR, 等于 §5.2 跟 §5.3 拼不起来。
- 修法: §5.2 增 1 条 FR-206 "GET /api/laputa/proposals?since=ISO8601" (返 changed proposals 增量), 引用 FR-301 fallback; 或 FR-301 fallback 复用 FR-203 "snapshot?since" 但要明示 "含 proposals section"。

### H-4. Rollback 级联 (cascade) 语义未定义

- FR-403 (L188-190) 定义单条 rollback: "反向应用 before/after 修复 target_section, 写新 changelog (action=rollback, 引用原 changelog_id), 标原 changelog reverted=true"
- **未定义**:
  - changelog #1 写 A→B, changelog #2 写 B→C, 回滚 #1 应该是 B→A 还是 C→A? (state 是 B 还是 C?)
  - 回滚 #1 后, #2 仍标 "B→C applied", 但实际 state 是 A, #2 现在指代的是 "从未存在过的转换" — **changelog 历史跟 state 脱节**。
  - 连续回滚 2 条 (先 #2 再 #1) 跟 1 次 "rollback to before #1" 行为不同, PRD 没说哪种对。
  - 回滚 #1 后, "rollback #2" 还允许吗? (30 天窗口外 #1 不可回滚, 但 #2 还在窗口内)
- 后果: 用户走 UI 选 "回滚到 N 天前", 后端要决定从最新 changelog 倒序回滚还是直接写 "restore to snapshot" — PRD 不规定行为, 工程师各自实现, 行为不一致。
- 修法: FR-403 增 1 条子要求 "rollback MUST 倒序应用 (从最新到最旧), 中间所有 changelog 一并 reverted=true; 30 天窗口外的 changelog 在链中则整个 cascade 返 Err::RollbackExpired"。

### H-5. §3 Concerns 10 项 P0 + §0/§1/§2/§3 4 段占位符 = launch review 站不住

- §0 L27 "待 Coaching 第 0 步完成填写" — 占位
- §1 L33-43 是 state report 不是 vision (walker 已标)
- §2 L47 "待 Coaching 第二节填写" — 占位
- §3 L57-68 列 10 项 P0 候选, **全标 P0 等于没标** (per walker [high] finding)
- 后果:
  - launch review 上 PM 拿不出 "Laputa 解决什么 user problem" 1 句话 (review-rubric.md [high] §1 Vision finding);
  - D-006 L138 锁 "Q2 受众 = 全 (实现者 + 架构师 + 文档读者)", 但 §2 是占位, 3 类受众想要的 PRD 形式没差异 (e.g. 实现者要 file:line 引用, 架构师要 thesis, 文档读者要 user 视角);
  - §3 10 项 P0 无答复, 跟 §5 已详 FR 形成强对比, reader 误判 "上半不重要"。
- 修法: §0/§2/§3 polish 阶段填实; §1 加 1 段 thesis (walker [high] 已给蓝本); §3 10 P0 用 3 tier 标 (P0 留 3 项 / P1 4 项 / P2 3 项), 跟 §6 OQ 关/半关/开 分布对齐。

---

## MEDIUM (6+ 条, polish 阶段处理)

### M-1. §6 OQ #4 (L290) 写权契约 跟 FR-501 schema 字段脱节

- FR-501 (L201) 列 `write_authority: "agent_self" | "user_only" | "shared"` 3 值枚举
- 14 section 没一格填这个字段
- §6 OQ #4 仍开, 没承诺 PR
- 关联 critical C-1

### M-2. FR-501 `render_layer` 枚举无设计意图

- L200 注释 "per A §22 3 值枚举, 注: 跟 §21 L0-L4 5 层不同, 用 3 值简化"
- L200 字段 3 值: always / relevant / archive / tbd
- **缺**: 14 section 哪几格填 always (e.g. identity? commitment?)? 哪几格填 relevant? relevant 怎么"相关" (per turn 关键词? per session 主任务?)? archive 何时降级 (时间窗? size 阈值? 用户手动?)
- reconcile-auto-evolution.md §5 actionable #4 同源, walker [medium] §5.5 已标
- 后果: 工程师实现 #1 identity 渲染逻辑时, 不知道 "always" 是 "每条 chat 都带" 还是 "agent 自决定带", 行为不一致

### M-3. FR-503 迁移幂等性 / 部分失败恢复路径未定义

- FR-503 (L218-224) step 1-4
- **未定义**:
  - 第二次启动 .laputa/state.json 已存在 + 旧 8 文件也还在, 走 step 3 "增量迁移" 还是 step 1 "存在 → 跳过"?
  - 启动期 .laputa/state.json 写入一半崩溃, 下次启动怎么 resume? (atomic write? 备份?)
  - 旧文件内容是 corrupt (e.g. UTF-8 非法, 截断) 怎么办?
- 关联 walker [medium] §5.5 finding

### M-4. FR-701 性能 SLA 在 4 道治理下不可达

- FR-701 (L257-258) 写: "Laputa 写 MUST ≤50ms (mock 数据 100 行)"
- FR-103 (L115-122) 4 道: review + changelog 写 + state 写 + audit 发 — 单次 file IO 4 次 + lock + diff 计算, mock 50ms 紧, prod 数据 100kb+ 时 50ms 不可达
- 后果: SLA 必破, 验收跑不过 → FR 标 "未达成", launch 阻塞

### M-5. BOOTSTRAP 段无 API 入口

- FR-505 (L231-232) 写: "BOOTSTRAP.md 是首次启动引导 (1 次性), MUST 不入 Laputa 持久层"
- 14 section 表格 (§4.1 L77-81) 不含 BOOTSTRAP
- FR-202 (L144) 14 section name 不含 BOOTSTRAP
- **后果**: 启动期谁读 BOOTSTRAP.md? 注入到哪儿? 用 mtime 检 (L232) 的话, mtime check API 在 §5.2 哪条?
- §6 OQ #11 (L297) "BOOTSTRAP.md 内容来源 (谁写首次启动引导? 仓库内置 / 远程拉?)" 仍半关

### M-6. FR-603 escape hatch 机制无实现锚定

- L250 注释: "'always' 是 escape hatch 仅供调试用, 不进 enum"
- **未说**: escape hatch 怎么开? (env var? 配置文件? debug build flag?)
- 后果: 后人想"在 prod 临时开 always 调试" 无 PRD 指引, 可能反向开成 prod default

---

## LOW (机械性, 不驱动 verdict)

- **Glossary drift**: "MEMORY.md" (L77) vs "memory_md" (L195); section 命名两套 (#10-14 编号 vs FR-202 snake_case); "changelog" 一词二义 (#12 section vs FR-4xx audit log API)。walker [medium] Downstream usability 已标
- **0 个 `[ASSUMPTION]` / `[NOTE FOR PM]` tag** (整 PRD grep 0 命中), walker 已标
- **§6 OQ #12-#18** 7 项 follow-up PRD 引用, 缺每个 follow-up PRD 的入口 contract, 跨 PRD 引用易断章
- **FR-704** (L272) "100 个并发写 14 section, ≤200ms 总" 缺 P99 分位
- **FR-2xx/3xx/4xx response schema** 全 markdown 表格 + 示例字段, 缺 JSON schema 锚定, code review 不易查
- **Addendum** 段 (L312) "(空 — 材料汇总后由子 agent extract 落入)" 是 shape noise

---

## 跟兄弟 PRD 的隐含冲突 (单列, 不混 high/medium)

1. **prd-selfinprove**: D-011 锁 "实接" 但 selfinprove FR-5xx stub 拆法没在本 PRD 锁, 是 C-3 根因。Sprint Change Proposal 拆 stub 的入口契约 (哪几条 stub / 怎么换成 LaputaWrite) 无 FR。
2. **prd-autodream**: §1 Vision (L41) "AutoDream 只读消费方 (FR-7 Orient)" — 但 reconcile-mentle-laputa.md §7 指出 AutoDream 8 步 flow 含 "可选写 proposal inbox" + "可选写 Mentle 中间笔记", 跟"只读"冲突。本 PRD 应明示 "只读" 边界 = 读 Laputa 路径, 写仍走 proposal_inbox (走 LaputaWrite) — 这条断言当前在 PRD 零覆盖。
3. **prd-report-system**: D-005 §2 锁 "Report System 第 2 写消费方", reconcile-mentle-laputa.md actionable #3 指出 pending.jsonl 降级契约 (Report System 写失败时回灌) 在 PRD 零覆盖。后果: Report System 写 Laputa 失败时, 数据存哪? 谁负责 retry? 沉默即丢。

---

## 行动建议 (给 polish 阶段, 优先级)

1. **P0 (必须修才 ship)**: C-1 + C-2 + C-3, 3 真 phase-blocker
2. **P1 (polish 必做)**: H-1 (audit 不可达) + H-2 (grep 绕过) + H-3 (fallback 端点) + H-5 (framing 占位)
3. **P2 (后续小 PRD)**: H-4 (rollback cascade) + M-1~M-6 + 兄弟 PRD 隐含冲突 3 条

修完 3 critical 后, 本 review 即可从 BLOCK-pending-patch 升 adequate-to-ship (跟 walker 终判 adequate 对齐, 但 adversarial 侧需明确 phase-blocker 闭环)。
