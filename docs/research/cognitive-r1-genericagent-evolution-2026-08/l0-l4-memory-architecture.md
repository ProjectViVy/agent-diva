# L0–L4 Memory Architecture（GenericAgent）

- 基线：`ee5a474`
- 命名红线：**L0 ≠ L1**（META-SOP ≠ insight 索引）

## 1. 层级总表

| 层 | 名称 | 载体 | 注入 | 职责 |
| --- | --- | --- | --- | --- |
| **L0** | META-SOP | `memory/memory_management_sop.md` | 结算时全文；非常驻 | 写记忆宪法 |
| **L1** | Insight Index | `memory/global_mem_insight.txt` | 每任务 system + 每 10 turn | ≤30 行存在性索引 + RULES |
| **L2** | Global Facts | `memory/global_mem.txt` | 按需 file_read | 环境事实（路径/配置/凭证） |
| **L3** | Task SOPs/Skills | `memory/*_sop.md`, `*.py` | 按需 file_read | 任务坑点/可复用流程/脚本 |
| **L4** | Session Archive | `memory/L4_raw_sessions/` | 不自动进 prompt | 压缩历史 + 可选 salient mining |

注入脚手架（非 L1 文件本体）：`assets/insight_fixed_structure*.txt`（路径图 + CONSTITUTION）。  
L1 种子：`assets/global_mem_insight_template*.txt`（首次启动复制）。

## 2. Bootstrap（`agentmain.py`）

```text
mkdir memory/
if missing global_mem.txt → "# [Global Memory - L2]\n"
if missing global_mem_insight.txt → copy template
system prompt = sys_prompt + Today + get_global_memory()
```

`global_mem*.txt` 通常 gitignore；运行期产物不随仓库分发。

## 3. L1 规则（存在性编码）

- **硬（SOP 声称）：** ≤30 行；**<1k tokens** 期望  
- 内容：高频 key→value；低频文件名列表；`[RULES]` 红线/高频错  
- 禁止：密码、How-to 细节、任务技术细节、日志、括号内参数值  
- 更新：L2/L3 结构变化时同步**名称/关键词**，不同步细节  
- cleanup：`memory_cleanup_sop.md` ROI；幽灵指针审计；词级 patch only

## 4. L1 ↔ L2/L3 同步表

| L2/L3 操作 | L1 |
| --- | --- |
| 新增场景 | 加文件名（默认低频）；反直觉触发词才加括号 |
| 删除 | 删映射行 |
| 改值且定位不变 | 不动 L1 |
| 通用避坑 | 压成 1 句进 RULES |

## 5. L2 / L3

**L2：** `## [SECTION]` 事实；可膨胀；冗长段可迁 L3 后 L2 留 6–9 行指针。  
**L3：** 只记跨会话重要且难重建的坑/前置条件；SOP 极简；脚本封装复杂复用。

## 6. 分类决策树

```text
环境特异性事实? → L2 (+ L1 索引)
通用操作规律? → L1 RULES (1 句)
特定任务技术? → L3
否则 → 丢弃（常识/冗余）
```

## 7. L4 流水线

| 项 | 事实 |
| --- | --- |
| 源 | `temp/model_responses/*.txt` |
| 周期 | scheduler **12h**（`_l4_t > 43200`） |
| 代码 | `compress_session.batch_process` |
| 产物 | 压缩 session 文件、`all_histories.txt`、月度 zip |
| 保留 | 最新 10 个 raw；跳过 mtime&lt;2h；过小压缩跳过 |
| Salient mining | **SOP 驱动**，非 12h cron 内置 |

L4 **不**自动升格为 L1/L2 事实。

## 8. Project Mode 并行层

| 项目术语 | 路径 | 与全局 |
| --- | --- | --- |
| Project L1 | 注入指针 + 规则 | ≠ `global_mem_insight.txt` |
| Project L2 | `temp/projects/<name>/project_memory.md` | ≠ `global_mem.txt` |

## 9. Working 边界

`update_working_checkpoint` **不是** L0–L4；会话内便签。不得替代 L2/L3 存环境事实。

## 10. 读/写/触发矩阵

| 层 | 主读 | 主写 | 触发 |
| --- | --- | --- | --- |
| L0 | 结算工具 | 维护者/极少 patch | 规则演进 |
| L1 | get_global_memory | file_patch | 结构变化/cleanup |
| L2 | file_read | file_patch | 验证后事实 |
| L3 | file_read / import | create/patch | 硬赢经验 |
| L4 | 人/agent/miner | batch_process | 12h cron |

## 11. 对 Diva 的含义（建议）

1. 保留 **L0 治理 ≠ L1 索引** 命名。  
2. Always-on 预算应是极简索引，不是全文 Memory/Skill。  
3. L2 环境事实应对齐 **Memory/BML**，L3 能力应对齐 **Evolution/Skill** — 勿再捆进同一 proposal 类型。  
4. L4 是归档/挖掘，不是 Skill 权威。  
5. Project 作用域记忆是第二系统，映射时勿与全局 STM/BML 混淆。
