# Acceptance — GA 功能对齐 × Memory / Laputa / AutoDream

> 本清单用于判定“功能性完全对齐 + 三件套完全可用”。  
> 当前迭代仅交付盘点文档，**以下条目全部未勾选验收**（预期）。

相关：`inventory.md` 域 ID；`verification.md` 证据。

---

## 0. 元验收（本迭代）

- [x] 已产出完整残缺项盘点（`inventory.md`）
- [x] 已按域（A–I）列出状态与优先级
- [x] 已定义用户旅程 U1–U8
- [x] 已定义落地波次 Wave 0–5
- [x] 已声明非目标（file_write 即权威等 🔒）
- [ ] 产品决策冻结（见 inventory §10 Open Questions）— **实施前必须完成**

---

## 1. Memory Agent 面（域 A / C / D / E）

### 1.1 即时管理（P0）

- [ ] 用户说“记住 X”，Agent 调用正式 memory 工具（非口头、非误用 write_file）
- [ ] 工具返回明确状态：`applied` **或** `proposal_created`（含 proposal id）
- [ ] **禁止**返回含糊“已记住”却无权威/无提案
- [ ] 用户说“忘掉 X”，默认 prompt 不再出现 X（tombstone 或等价）
- [ ] `list` / `read` 与 applied authority 一致
- [ ] `search` / 显式 recall 工具在跨会话可用
- [ ] system prompt **不**承诺不存在的 memory tools

### 1.2 工作记忆（P0）

- [ ] 存在 `update_working_checkpoint`（或等价）工具
- [ ] 回合上下文可注入 WORKING MEMORY 块
- [ ] working 写入 **不**直接成为 long-term authority
- [ ] 存在 working → long-term 的显式晋升路径（经治理）

### 1.3 主动蒸馏（P0）

- [ ] 任务结束可调用 distill / start_long_term_update 等价物
- [ ] 蒸馏遵循 Action-Verified / 最小 patch 纪律（可观测）
- [ ] 不再仅依赖 ≥100 消息的 consolidation 作为唯一写路径

### 1.4 分层精神（P0–P1）

- [ ] 启动注入有 L1-like 硬预算（不全文灌入）
- [ ] 细节依赖 search/read 按需加载
- [ ] 存在 L0 管理策略（内置 policy 或可配置 SOP）
- [ ] 写入拒绝明显 volatile 内容（可配置策略）

### 1.5 召回（P0）

- [ ] Typed 模式 prefetch/FTS 在生产配置下可注入
- [ ] Legacy / file-first 模式不静默 Failed 却无用户可理解降级
- [ ] 召回结果带 trust/provenance 标注

---

## 2. Laputa 治理层完全可用（域 F / H）

- [ ] Agent / AutoDream / GUI 记忆变更均可进入统一 proposal 列表
- [ ] 批准 apply 后：section 与 typed store 一致
- [ ] 拒绝后：不进 authority，且可被 suppression 抑制原样重提（若启用）
- [ ] rollback 可恢复到 changelog 前状态
- [ ] high-risk memory 无 evidence 不可批
- [ ] degraded（`.laputa`/typed 损坏）不静默回落 Markdown 冒充权威
- [ ] `authority_mode` 缺省/默认语义统一，文档与出箱行为一致
- [ ] Apply 后新会话（或已定义的同会话刷新策略）可见更新
- [ ] pending 默认不进入 system prompt authority

---

## 3. AutoDream 完全可用（域 G / H）

- [ ] 手动触发：完整 run → 终态（completed / failed / no_candidates / cancelled）
- [ ] 失败有稳定错误码与事件，可在 GUI 观察
- [ ] 合格候选一对一进入 Laputa PendingReview
- [ ] 自动阈值（session/消息）在配置开启时可靠触发
- [ ] 日报/周报/月报可生成且 GUI 可打开
- [ ] 不阻塞主聊天回路
- [ ] 与 agent 即时记忆分工文档化且实现一致（即时 tool vs 节律蒸馏）
- [ ] CandidateGate 拒绝无证据/低价值/注入等（已有则回归通过）

---

## 4. 用户旅程端到端（U1–U8）

| ID | 旅程 | 通过条件 |
|----|------|----------|
| U1 | 记住我叫… | 工具成功 + 状态诚实 + 可追踪 |
| U2 | 你还记得… | 跨会话能答对或明确“无记录” |
| U3 | 忘掉… | tombstone 后默认不可见 |
| U4 | 任务结算经验 | distill 可触发且产提案/条目 |
| U5 | 长任务 checkpoint | working memory 不丢关键点 |
| U6 | 节律自动挖记忆 | AutoDream 自动路径 + 提案 |
| U7 | GUI 批准提案 | apply 后对话权威更新 |
| U8 | 查看我的记忆 | PersonaMemory 非空且与 authority 一致 |

- [ ] U1
- [ ] U2
- [ ] U3
- [ ] U4
- [ ] U5
- [ ] U6
- [ ] U7
- [ ] U8

---

## 5. 非回归 / 安全

- [ ] 不恢复 Mentle 运行时依赖
- [ ] 模型不能通过 `write_file` 绕过治理成为 authority（若仍可写文件，须 fail-closed 或明确非权威）
- [ ] 身份/承诺等高敏变更仍需审批
- [ ] 密钥/凭证不进入 L1 式常驻索引

---

## 6. 本迭代文档验收（通过）

| 项 | 结果 |
|----|------|
| inventory 覆盖 GA L0–L4 / tools / working / reflect | Pass |
| inventory 覆盖 Diva Memory/Laputa/AutoDream 现状与证据路径 | Pass |
| 优先级与波次可指导实施 | Pass |
| 无代码变更 | Pass |
