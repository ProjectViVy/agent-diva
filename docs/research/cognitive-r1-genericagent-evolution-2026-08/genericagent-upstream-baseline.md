# GenericAgent Upstream Baseline

- 状态：`Research Source Fact`
- 日期：2026-08-13
- 本地树：`C:\Users\Administrator\Desktop\morediva\.workspace\genericagent`
- 上游：`https://github.com/lsdefine/GenericAgent`

## 1. 可复现身份

| 项 | 值 | 证据 |
| --- | --- | --- |
| Branch | `main` | 源码事实 |
| Local HEAD | `ee5a474e5a1b6d203438d7d1dfa21da912268bb8` | 源码事实（2026-08-13 `git rev-parse`） |
| Local HEAD date | 2026-07-10 | 提交事实 |
| Local HEAD message | `fix: isolate project mode per agent instance` | 提交事实 |
| Local `origin/main` tracking | 同 `ee5a474` | 源码事实 |
| Working tree | clean vs tracking（`main...origin/main` 无 ahead/behind 标记） | 源码事实 |
| `mykey.py` | 不存在 | 源码事实 |
| `skills/` | 不存在 | 源码事实 |
| Remote tip（GitHub REST，2026-08-13） | `f06d5503808ba9d164fb583e4c500d5ce01efd4c` | 提交事实 |
| Decision-record 曾记远端 | `63f9db74e63fef54950ed7f6f43e43295fb6b36b` | 仍为 tip 祖先 |
| Content twin of local HEAD on remote line | `9d99e9fc74de216e54fb222380b91c0c69026109`（同 tree） | 提交事实 |
| 本地 `git ls-remote` | 曾 SSL handshake 失败；以 API 补证 | 源码事实 / 限制 |

**基线声明（研究锁定）**

```yaml
local_baseline: ee5a474e5a1b6d203438d7d1dfa21da912268bb8
remote_main_as_of_2026-08-13: f06d5503808ba9d164fb583e4c500d5ce01efd4c
content_equivalent_remote: 9d99e9fc74de216e54fb222380b91c0c69026109
skills_directory: ABSENT
skill_storage: memory/*_sop.md + memory/*.py
```

**推断：** 上游在 7 月中旬后对 main 做过 rebase/祖先重写；本地与远端同内容不同 SHA。研究以**本地树源码**为主，远程 tip 作 delta 对照。

## 2. README “Skill” vs 仓库实现

| README 叙事 | 仓库实际 | 标签 |
| --- | --- | --- |
| Automatically crystallizes Skill | `start_long_term_update` → 模型 `file_patch` L1–L3 | 源码事实 |
| Skill tree | L1 索引 + `memory/` 文件集合 | 源码事实 |
| L3 = Task Skills / SOPs | `memory/*_sop.md` 与 helper `.py` | 源码事实 |
| `skills/` 包目录 | **不存在** | 源码事实 |
| Skill Marketplace / Sophub | 外链 / coming soon；无 loader | 源码事实 |
| Don't preload skills | 仓库**预装**大量 SOP | 源码事实 + 推断（营销张力） |

**CONTRIBUTING 分流（源码事实）**

| Type | Destination |
| --- | --- |
| Fundamental | Core `memory/` |
| Domain-specific | Skill Marketplace *(coming soon)* |

结论：**Skill ≈ L3 记忆文件**，不是 Diva 式 `skills/<name>/SKILL.md` 包，也不是独立领域类型。

## 3. 进化机制因果图

```text
9 atomic tools + agent loop
        │ task execution
        ├─ update_working_checkpoint  → session working (non-durable)
        ├─ start_long_term_update     → inject L0 + distill prompt (no write)
        │         └─ model file_patch → L2 facts / L3 SOP / L1 index
        ├─ reflect/scheduler          → L4 archive 12h + sche_tasks
        ├─ autonomous / goal_mode     → offline loops (often skip LTM)
        └─ project_mode plugin        → per-project memory (parallel L1/L2)
```

**源码事实：** 不存在 `save_skill()` API。结晶 = 在 L0 约束下对 `memory/` 的最小 patch。  
**推断：** 质量高度依赖模型遵守公理 → “prompt-governed evolution”。

## 4. 主题提交时间线（摘要）

### 4.1 `start_long_term_update`

| 时间 | SHA 短 | 事件 | 本地含? |
| --- | --- | --- | --- |
| 2026-02 | `708ffb32` | `trigger_memory_update` → `start_long_term_update` | 祖先 |
| 2026-03 | `1396794a` | schema 15-turn 规则 | 是 |
| 2026-04 | `f40eaa9c` | 仅任务完成后调用 | 是 |
| 2026-07-22 | `733615d8` | **turn&lt;10 硬拦截** | **本地后**（远程） |

### 4.2 L4 / cleanup / project mode

| 主题 | 关键节点 | 本地含? |
| --- | --- | --- |
| L4 archiver + 12h cron | `5c538540` (2026-04) | 是 |
| `memory_cleanup_sop` | `d1e396fa` / `dd5c795a` | 是 |
| Project mode 插件 | #591/#592 → 实例隔离 `ee5a474` | 是（HEAD） |
| L3 no project-specific facts | `308153b1` (2026-08-10) | 本地后 |
| L1 RULES 放宽 | `f06d550` (2026-08-13) | 本地后 |

### 4.3 本地 HEAD (`ee5a474`) 自身影响

1. `handler.enter_project_mode(name)` 实例级状态  
2. Working history 注入改为隔 turn / 隔 4 turn  
3. 超长 write（>5000）附加 hallucination 警告  

## 5. 关键文件清单（演进研究）

| 路径 | 角色 |
| --- | --- |
| `ga.py` | 工具、working/LTM、anchor、global mem |
| `agent_loop.py` | turn 循环、tool_results |
| `agentmain.py` | bootstrap、BANNED_TOOLS、reflect |
| `llmcore.py` | 会话/trim |
| `assets/tools_schema*.json` | 9 工具契约 |
| `assets/sys_prompt*.txt` | 系统角色 |
| `assets/global_mem_insight_template*.txt` | L1 种子 |
| `assets/insight_fixed_structure*.txt` | 结构 + CONSTITUTION |
| `memory/memory_management_sop.md` | **L0** |
| `memory/memory_cleanup_sop.md` | L1/L2 GC |
| `memory/*_sop.md` / `*.py` | **L3 skills** |
| `memory/L4_raw_sessions/*` | L4 |
| `reflect/*` | 调度/自主/goal |
| `plugins/project_mode.py` | 项目记忆 hook |
| `CONTRIBUTING.md` / `README.md` | Skill 叙事 |

本地 `memory/*_sop.md` 计数：**22**（2026-08-13）。

## 6. 与 Diva 的映射提示（建议 only）

| GA | Diva 可能对应 | 建议 |
| --- | --- | --- |
| L0 META-SOP | Skill 写入策略 / 内容门槛 | 写前纪律 |
| L1 Insight | Skill 目录极简索引 | 控 token |
| L2 Facts | 环境事实（属 Memory，非 Evolution） | 与 Skill 隔离 |
| L3 SOP/script | Skill 文档 + 可选实现 | **Evolution 对象** |
| L4 Archive | 会话归档/挖掘 | 非 Skill 权威 |
| `start_long_term_update` | 任务结束结晶触发 | 对象应为 Skill 候选，不是 Memory proposal |
| 无 `skills/` 目录 | Diva 已有 `skills/*/SKILL.md` | 勿 1:1 镜像 GA 路径；可对齐**哲学** |

## 7. 开放问题

1. 本地 fetch 成功后 merge-base 是否确认 rebase？  
2. `733615d8` turn&lt;10 拦截是否仍在 `f06d550`？  
3. Sophub 包格式与 `memory/` 是否互通？  
4. L4 salient mining 是否有 cron，还是仅 SOP 触发？（scheduler 仅 compress）  
5. 远程 tip 相对本地的 evolution 行为 delta 需文件级 diff 二次核对。
