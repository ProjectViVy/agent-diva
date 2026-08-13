# Action-Verified Evidence Model（GenericAgent）

- 基线：`ee5a474`
- 核心文件：`memory/memory_management_sop.md`、`ga.py`、`agent_loop.py`

## 1. 四条核心公理（L0 文本）

| 公理 | 操作定义 | 代码强制？ |
| --- | --- | --- |
| **Action-Verified Only** | L1/L2/L3 写入必须源自成功工具结果；禁固有知识/猜测/未执行计划 | **否** — 仅 SOP + 结算 prompt |
| **Sanctity of Verified Data** | 已验证事实可压缩/迁移，不可 GC 丢弃；优先小 patch | **否**（patch 唯一匹配是结构约束，非语义） |
| **No Volatile State** | 禁时间戳/临时 session/PID/会话临时路径等 | **否** — 无分类器 |
| **Minimum Sufficient Pointer** | 上层只留最短定位符 | **否** — L1≤30 仅 SOP 自称硬约束 |

口号：**No Execution, No Memory.**

## 2. 强制矩阵

| 机制 | 标签 | 说明 |
| --- | --- | --- |
| 工具结果进入下一轮 `tool_results` | **代码强制** | 模型能看见结果 |
| `file_patch` 唯一块匹配 | **代码强制** | 不保证内容真实 |
| memory 路径写保护 | **缺失** | 任意路径可 patch/write |
| 写入绑定 tool_use_id / success | **缺失** | 无 evidence_refs |
| volatile/secret 过滤器 | **缺失** | 纯纪律 |
| `start_long_term_update` 注入 L0 | **混合** | 提高遵守概率，不强制后续 patch |
| plan 缺 VERDICT 拦截 | **软** | 仅 plan 完成叙事，非记忆门 |
| L1 禁密码 | **SOP** | L2 决策树却可含凭证 — 张力 |
| keychain.py | **代码能力** | 旁路密钥存储，不拦截 L2 明文 |

## 3. 证据链（实际）

```text
LLM tool_call → physical effect → tool_results 回注
  → 模型判断是否长期记忆
  → (可选) start_long_term_update 注入 L0
  → file_read 现有 → file_patch L1/L2/L3
  → 无 success gate / 无 evidence_ref
```

**关键结论（源码事实）：**

> Action-Verified **不是**写入 API 的前置条件，而是写记忆时的**提示词宪法**。  
> 模型可随时直接改 `global_mem*.txt` / SOP，无需证明 tool success。

## 4. 失败 / 假设 / 敏感 / 易变

| 类别 | 过滤 |
| --- | --- |
| 失败工具结果 | 回注；应不写 L*；无人阻止写坏结论 |
| 未验证假设 | 文本禁止 |
| “做了没验证” | 结算 prompt 禁止 |
| 通用常识 | 决策树丢弃 |
| Volatile | 公理文本 |
| Secrets | L1 禁；L2 可；keychain 旁路 |

## 5. Working vs Long-term

| 通道 | 工具 | 持久化 | Action-Verified |
| --- | --- | --- | --- |
| Working | `update_working_checkpoint` | 进程内 | 不要求 |
| Long-term | `file_*`（经/不经结算） | 磁盘 L1–L3 | 仅纪律 |
| L4 | compress + mining | 归档 | 非工具验证事实；salient 用 user 原文 |

## 6. 缺失能力（相对可审计进化）

- `evidence_refs[]` → tool_use_id  
- 写入时 resolve success  
- proposal/审批（GA 直接写盘）  
- 假设状态机 hypothesis→verified  
- 自动化测试（仓库 **无** test suite 强制公理）

## 7. 对 Diva 的建议（非架构批准）

| 概念 | 建议 |
| --- | --- |
| evidence_refs | Skill/长期写入候选强制绑定 tool 证据 |
| tool-result binding | apply 前 resolve success |
| Working | 可无 evidence（对齐 GA working） |
| 权威层 | 未验证不可进 Skill/BML 权威 |
| 不要 | 宣称 “prompt 写了 No Execution = 已实现证据模型” |

**若 Diva 照搬 GA 而不加代码门：同样无法证明 Action-Verified。**
