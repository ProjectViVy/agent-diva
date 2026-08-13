# Evolution Behavior Experiments

- 日期：2026-08-13
- 基线：`ee5a474` @ `.workspace/genericagent`
- 模式：P0 静态（完成）；P1/P2 活体（阻断）

## Methods

### P0 静态

1. 解析 `assets/tools_schema.json` / `tools_schema_cn.json`
2. 追踪 `ga.py`：`do_start_long_term_update`、`file_patch`、working、global mem
3. 交叉 `agentmain.BANNED_TOOLS` / `--no-user-tools`
4. 搜索 Action-Verified 测试（`test_*.py` / pytest）
5. 区分代码强制 vs 模型依赖

### P1/P2 门禁

| 条件 | 观测 |
| --- | --- |
| `mykey.py` / `mykey.json` | **缺失**（仅 template） |
| 隔离 temp 记忆沙箱 | 可行但未执行 |
| 不写生产 `global_mem*` | 策略就绪未执行 |

## P0 Results

### 1. 工具 schema（记忆相关）

| 工具 | 参数 | 要点 |
| --- | --- | --- |
| `file_read` | path, start, count… | 修改前读取 |
| `file_patch` | path, old, new | 唯一精确匹配 |
| `file_write` | path, content, mode | 大改；应少用 |
| `update_working_checkpoint` | key_info, related_sop | 短期；非收尾 |
| `start_long_term_update` | **{}** | 开启结算；schema 称 15+ turns 必调 |

共 9 原子工具；`no_tool` 为引擎伪工具。

### 2. `do_start_long_term_update` 可观察契约

**代码路径（源码事实）：**

1. 构造中文蒸馏 `next_prompt` + `get_global_memory()`
2. yield Info 行
3. 读 L0 → tool_result；缺失则 “Do not update memory.”
4. **零次文件系统写记忆**

**结算指令核心（摘要）：** 只提行动验证成功且长期有效；L2 事实 / L3 SOP；禁止临时/未验证/常识；先 read 再最小 patch。

**脆弱点：** L0 路径 `./memory/...` 依赖进程 CWD；`GA_LANG=en` 时结算 prompt 仍为中文。

### 3. BANNED_TOOLS

```python
BANNED_TOOLS = ['ask_user', 'start_long_term_update']  # if --no-user-tools
```

UltraPlan 子代理默认开启。禁的是 schema 暴露，不是方法删除。

### 4. 测试

**n_test_files_enforcing_action_verified = 0**  
无 pytest suite；公理无自动化回归。

### 5. Claimed vs Observed

| 宣称 | 观测 | 判定 |
| --- | --- | --- |
| L0–L4 结构 | 文件与注入存在 | 确认结构 |
| LTM 工具蒸馏记忆 | 仅启动结算过程 | 部分真 |
| Action-Verified Only | 仅 prompt/SOP | **未强制** |
| 15+ turns 必调 | 仅 schema 文案 | **未强制** |
| memory 只能 patch | 软；write/code_run 仍可用 | **未强制** |
| 9 tools | 确认 | 确认 |
| 子代理 skip LTM | `--no-user-tools` | 确认 |
| Working 自动注入 | `_get_anchor_prompt` | 确认 |
| file_patch 唯一匹配 | 代码 | 确认 |
| 公理测试 | 无 | 缺失 |

### 6. 端到端 LTM 流

```text
Model optional call start_long_term_update
  → [Code] inject L0 + rules
  → Model may file_patch
  → [Code] string/fs rules only
  → No Action-Verified verifier
```

## P1 / P2 Status

**BLOCKED：** usable_key_files = 0。

安全 P1 设计（未跑）：

1. 进程外密钥，不落库  
2. 复制 L0 到 temp 记忆沙箱  
3. 对比“有工具成功证据 vs 纯幻觉”写入率  
4. 日志仅 `temp/model_responses`

P2 预测：

| 探针 | 静态预测 |
| --- | --- |
| 无验证调用 LTM | 模型应跳过；代码仍注入 L0 |
| patch 不匹配 | 硬错误 |
| banned 标志 | 模型看不见工具 |
| 错误 CWD | L0 缺失文案 vs next_prompt 鼓励更新（矛盾软信号） |
| 直接 overwrite L1 | 代码允许 |

## Open Questions

1. Action-Verified 是否应在 Diva 变硬门？  
2. L0 路径应否 `script_dir` 绝对化？  
3. 15+ turns 是否加硬/软提醒？  
4. EN 中文结算 prompt 是 intentional 还是 bug？  
5. 无密钥时是否值得加纯 Python 单测：`file_patch` 唯一性、ban filter？
