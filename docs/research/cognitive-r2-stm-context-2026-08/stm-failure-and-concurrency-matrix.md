# R2 STM 失败与并发矩阵

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 性质：场景 × 当前行为 × 产品需求 × 缺口；不定恢复方案
- 「阻断 D2」= 设计必须先回答；不是本包去改代码

列说明：

- **当前**：今天真实发生的事（无 STM 时的邻近对象）
- **S3 需求**：产品 STM 在该场景必须满足的行为
- **缺口**：当前实现与需求的差
- **D2**：设计是否必须先拍板

## 1. Session 身份与可见性

| ID | 场景 | 当前 | S3 需求 | 缺口 | 证据 | D2 |
| --- | --- | --- | --- | --- | --- | --- |
| V1 | 新 GUI chat（新 `gui:{id}`） | 空 transcript；空 CanonicalCheckpoint；无 WM 行；L1 仍可见 workspace LTM | 必须看到**同一** STM 有界投影 | 无 STM 权威可投影 | admission `session_key`；L1 排除 session-scoped | 必须 |
| V2 | 同 key resume（重启后加载 JSONL） | transcript + CanonicalCheckpoint 恢复；WM 行若未被 GC 仍注入 | STM 必须恢复，且不依赖该 session 是否 compact 过 | WM 可能在；STM 无 | SessionManager.load | 必须 |
| V3 | 另一 channel（如 `cli:direct` vs `gui:x`） | 完全独立的 JSONL / checkpoint / WM | 同一 STM | 三套邻近对象都按 session 切开 | session_key 公式 | 必须 |
| V4 | Manager `api:default` 与 GUI 并行 | 两 session；共享 BML LTM | 同一 STM，写冲突要有规则 | 无合并 | 约束文 §5 | 必须 |
| V5 | 子代理运行中 | 不走 C1–C5；无 WM 工具；可见 Typed L1 | 读？写？默认应不覆盖父 STM | 未定义 | `subagent.rs` prompt；`!subagent_mode` | 必须 |
| V6 | 子代理完成后打回父 session | 父下一 turn 加载父 WM / checkpoint；子状态只有一条公告消息 | 子结论若进入 STM，必须是父权威上的受控更新 | 无快照合并 | inbound announce | 建议 |
| V7 | GUI cron | session=`api:cron:{to}`，**不**进 GUI 聊天 | cron 是写 workspace STM 还是 job-local | 未定义；隔离的是聊天 session | `runtime.rs` remap | 必须 |
| V8 | CLI 默认 cron | session=`cli:direct`，与交互 CLI **共享** history/WM/checkpoint | 不得让定时任务悄悄改交互 STM，除非显式 | 当前会共享邻近对象 | admission + cron default | 必须 |
| V9 | Heartbeat | 服务未接线；`HEARTBEAT.md` 未被 loop 读 | 不能假设节律维护 STM | 无 | `HeartbeatService::new` 仅测试 | 否（先保持休眠） |

## 2. 生命周期：清谁、留谁

S3/S4/最低测试：session checkpoint 结束清理 **不**删 STM；STM 清理 **不**删 BML 或 transcript。

| ID | 场景 | CanonicalCheckpoint | SessionCheckpoint (WM) | BML LTM | 产品 STM 应 | 缺口 | D2 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| L1 | AgentLoop 正常 shutdown | JSONL 留在磁盘 | `on_session_end` 物理 DELETE | 留 | 留 | STM 不存在；WM 被删是对的 | 否 |
| L2 | Reset（`archive_and_reset`） | 随 JSONL 改名归档 | **不删**；同 key 下轮仍注入 | 留 | 留；且不应被当成「新会话空白 scratch」 | WM 残留造成「新会话看见旧便签」 | 必须（session 语义） |
| L3 | Delete session 文件 | 文件删除 | **不删** → 孤儿行 | 留 | 留 | 孤儿 WM；shutdown GC 也扫不到 | 必须 |
| L4 | Stop / cancel | 丢 pending reactive checkpoint | 不删 | 留 | 留 | 邻近对象不一致 | 建议 |
| L5 | `run_startup_gc([])` 若被误调 | 不动 | **扫光所有** `session_id IS NOT NULL` | 不动（NULL） | 必须不动 | 若 STM 误用 session_id 列会被灭门 | 必须 |
| L6 | `run_startup_gc` 未接线 | — | 崩溃后 WM 残留 | — | STM 崩溃后仍在 | WM 残留；STM 尚无权威 | 建议（接线与否交给 D2） |
| L7 | Compact Auto 成功 | 原子替换 | 不动 | 不动 | 不动；投影可重读 | 若把 STM 写进 checkpoint body 会丢 | 必须（约束文 §8.4） |
| L8 | Compact 失败 | 保留旧 checkpoint | 不动 | 不动 | 保留上一份 STM | checkpoint 已满足自身；STM 无 | 必须 |
| L9 | BML 完整性失败 | 仍在 JSONL | 随 Typed 降级 | `DegradedMemoryProvider` | 取决于权威是否同库 | 同库则 STM 一起瞎 | 必须 |

## 3. 写入失败与过期覆盖

| ID | 场景 | 当前邻近行为 | S3 需求 | 缺口 | D2 |
| --- | --- | --- | --- | --- | --- |
| W1 | `checkpoint_write` 校验失败（空 session 或空内容） | 返回 Failed；不写 | 保留上一份 | WM 满足；STM 无 | 必须复用「失败不覆盖」 |
| W2 | `put` CAS 冲突 | 返回 Failed；旧行在 | 过期响应不得覆盖新 revision | 跨进程无锁，CAS 只在同进程可靠 | 必须 |
| W3 | 两进程同时 `put` 同 id | WAL + 5s busy；无文件锁；last CAS wins | 明确合并或明确丢弃 | 跨进程丢失更新 | 必须 |
| W4 | 用户 GUI 修正与 Agent 自动更新同时 | 无 STM GUI；WM 无 GUI 编辑器 | 用户修正直接生效，不走审批；不能被过期自动写打回 | 无冲突协议 | 必须 |
| W5 | 异步 STM 更新在 compact / 新 turn 之后返回 | 无此路径 | 过期 revision 拒绝 | 无 | 必须 |
| W6 | 半写 Markdown（若选文件权威） | 无 STM 文件 | 读到上一份完整文档 | 需原子替换（BML/JSONL 已有范式） | 若选 A |
| W7 | 无证据的 STM→BML | `memory_add` 可立即写 LongTerm，无 evidence 强制 | 不得把推测固化为 LTM | 现成误晋升面就在 `memory_add` | 必须（晋升入口） |

## 4. 并发写者地图

潜在写者（今天写邻近对象，明天可能写 STM）：

```text
主 AgentLoop turn          → update_working_checkpoint / compact / plan finalize
第二 channel 的同一进程     → 另一 session_key，同 BML 文件
GUI 用户（未来 STM 工作区） → HTTP → Manager → store
CLI 交互                    → cli:direct
CLI / GUI cron              → 可能共享或隔离 session
子代理                      → 今天不能写 WM；完成后打父 session
第二进程（第二 gateway/测试）→ 同盘 sqlite / jsonl，无跨进程锁
```

| ID | 写者组合 | 当前 | 风险 | D2 |
| --- | --- | --- | --- | --- |
| C1 | 两 channel，同一 AgentLoop | WM id 不同，不合并 | 若 STM 是单行，变同 id 互踩 | 必须 |
| C2 | GUI + CLI 两进程 | 两套 session；BML 无跨进程锁 | LTM 已有此风险；STM 若同库继承 | 必须 |
| C3 | cron + 交互（CLI） | 同 `cli:direct` | 定时 turn 改交互 WM/checkpoint | 必须 |
| C4 | cron + 交互（GUI） | cron 在 `api:cron:` | 邻近对象隔离；STM 若 workspace 级仍争用 | 必须 |
| C5 | Plan durable `set_active_plan` | workspace 单例，last wins | 证明 Plan 不能当 STM | 否（高冲突已记） |
| C6 | EphemeralPlanRegistry 两 session | 各有 draft | 与 durable 单例不一致 | 否 |

## 5. Prompt 装配失败

| ID | 场景 | 当前 | STM 需求 | 缺口 | D2 |
| --- | --- | --- | --- | --- | --- |
| P1 | WM 读取失败 | warn，跳过块 | 失败保留上一份**已注入或可重试的**投影，不得注入损坏体 | 动态区失败=缺席，不是保留缓存旧块 | 建议 |
| P2 | L1 空 / degraded | prefix 仍有 L0 政策 + 降级说明 | STM 缺席不得误用 L1 顶替 | 命名都叫 L1 时易混 | 必须（拆名） |
| P3 | 预算压力 | 先丢 Recall；WM Never | STM 投影超预算应在权威侧收敛，禁止装配侧静默 Drop 造成 GUI/模型分叉 | 无 STM 层 | 必须 |
| P4 | 放进 prefix 后频繁自动更新 | L1HotRefresh 破 cache | 可接受或不可接受必须拍板 | 选项文 §8 | 必须 |
| P5 | 裸 ContextBuilder 测试 | 默认 `MemoryManager` / MEMORY.md | 生产 Typed 路径必须单测 | 测试≠生产 L1 | 建议 |
| P6 | compact 把「下一步」写进 checkpoint | 模型可见摘要 | 不得成为唯一 STM 副本 | 约束文 §8.4 | 必须 |

## 6. 与删除面、治理面的交叉

| ID | 场景 | 当前 | 需求 | 缺口 | D2 |
| --- | --- | --- | --- | --- | --- |
| G1 | 删除 `memory_md` | 仍活；Persona 左栏 / 提案 / 非默认 Prompt | STM 不得落在这条链 | S2 | 否（已冻结） |
| G2 | STM 日常写走 Proposal | Memory 更新/删除已走提案 | **禁止** | 旧链仍在，新对象不能接 | 必须（接线白名单） |
| G3 | Approval `domain=memory` | GUI remove 仍建提案 | STM 修正不进 Approval Center | 无 STM | 否 |
| G4 | BML 边界门禁 | 治理不得直调 put/GC | STM 日常写若碰 sqlite，只能走允许面 | allowlist 扩面是设计题 | 若选 B/C |

## 7. 最低测试需求对照（决策记录 → 本矩阵）

| 决策记录要求 | 覆盖 ID | 今日是否可测 |
| --- | --- | --- |
| 删除 `memory_md` 后零残留 | G1；交给 R4 | 否（R4） |
| BML 仍是唯一 LTM | 贯穿 | 是（R0 + E5） |
| 新 session / 重启 / 两 channel 同一活动集 | V1 V2 V3 | **否**（无 STM） |
| session checkpoint 清理不删 STM；STM 清理不删 BML/transcript | L1–L6 | 无 STM；反向：Reset/Delete 还不删 WM |
| 有界投影，不整灌历史/BML/WORLD/transcript | P3 + 约束文 §8.7 | 负向不变量部分已有 |
| 自动更新失败保留上一份；过期不覆盖 | W1 W2 W5 L8 | WM 部分满足 |
| 用户修正不建 Approval | G2 G3 | 无 STM GUI |
| Memory 页唯一入口 | 选项文 §11 | 无入口 |
| workspace gates + GUI + 真机跨会话 smoke | E9/E10 | 未跑 |

## 8. 推断出的设计必答题（不是方案）

1. STM 的主键是 workspace、profile，还是合成键？如何避免 `cli:direct` cron 污染？
2. Reset/Delete 对 SessionCheckpoint 是否补 `on_session_end`？与 STM 必须正交。
3. `run_startup_gc` 接线时，过滤条件必须是「session-scoped scratch」，**不能**是「所有非空 session_id」。
4. 跨进程写是「禁止第二进程写 STM」还是「CAS + 拒绝过期」？
5. 子代理默认只读还是禁止见 STM？
6. 装配失败时：缺席、降级文案，还是注入上一份缓存投影？

本矩阵不回答以上问题。
