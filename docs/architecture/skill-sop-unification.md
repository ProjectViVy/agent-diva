# SOP-as-Skill 统一设计决策

## 决策

Agent Diva 不再建设独立的 SOP 文件格式、目录、加载器或运行时。SOP 是
Skill 的一种语义类型，继续使用标准 `SKILL.md` 目录结构，并完整复用现有
Skill 的发现、加载、依赖检查、安装、安全审查和执行路径。

该决策避免形成 `sops/` 与 `skills/` 两套内容相似但行为不同的系统。传统
Skill 保持原有含义；被标记为 SOP 的 Skill 仍然是合法 Skill，可以被不认识
Agent Diva 扩展字段的旧版加载器或其他兼容实现读取。

## 建议格式

在 `SKILL.md` YAML frontmatter 中增加一个可选、枚举化字段：

```yaml
---
name: release-agent-diva
description: Validate, approve, and release Agent Diva.
kind: sop
---
```

字段契约：

- `kind` 可选；缺省值为 `skill`。
- 第一阶段仅接受 `skill` 和 `sop`，未知值按普通 `skill` 处理并记录诊断，
  不得阻断旧 Skill 加载。
- `kind: sop` 只增加语义标签，不改变加载、选择、权限和执行行为。
- SOP Skill 的正文仍遵循普通 `SKILL.md` 规范，可以引用脚本、模板、工具和
  其他资源。
- 不创建 `SOP.md`、`SOP.toml`、`workspace/sops/` 或第二套注册表。

选择顶层 `kind` 而不是从标题或描述中猜测，是为了提供稳定、低成本、可测试
的数据契约。实现时应保留未知 frontmatter 字段兼容性。

## 展示契约

识别到 `kind: sop` 时，在 Skill 名称附近增加一行小标：

```text
release-agent-diva
AGENT-DIVA SOP
```

中文界面可显示 `AGENT-DIVA SOP` 或 `Agent Diva 标准流程`，但底层标识始终
是稳定值 `sop`。普通 Skill 不显示额外标签，避免界面噪声。

该标签应逐步出现在：

1. Agent system prompt 的可用 Skill 摘要；
2. GUI 已安装 Skill 列表；
3. Skill 详情和安装预览；
4. 后续若存在 Marketplace 筛选，可作为筛选条件。

标签不是新的权限等级，也不能绕过现有 Skill 安全审查。

## 兼容性

| 输入 | 新版 Agent Diva | 旧版 Agent Diva / 传统 Skill 加载器 |
|---|---|---|
| 无 `kind` | 普通 Skill | 普通 Skill |
| `kind: skill` | 普通 Skill | 忽略未知字段后作为 Skill |
| `kind: sop` | 带 SOP 小标的 Skill | 忽略未知字段后作为 Skill |
| 未知 `kind` | 降级为 Skill并记录诊断 | 按原有兼容行为读取 |

不应批量修改已有 `SKILL.md`。只有确实表达稳定、可重复标准流程的新 Skill，
或经用户确认的既有 Skill，才添加 `kind: sop`。

## 实现范围

建议拆成一个小型、聚焦的后续迭代：

1. 在 `agent-diva-agent/src/skills.rs` 的 `SkillMetadata` 中加入强类型
   `SkillKind`，并解析可选 `kind`。
2. 在 Skill 摘要中为 SOP 输出稳定属性或元素，例如
   `<skill kind="sop">`，同时保留现有字段。
3. 将类型透传到 Manager/Tauri `SkillDto`。
4. 在 GUI 已安装 Skill 卡片中渲染 `AGENT-DIVA SOP` 小标。
5. Notebook 中原来的“固化为 SOP”语义改成“创建 SOP Skill 候选”，最终
   产物仍是带 `kind: sop` 的 `SKILL.md`。
6. 不在本迭代引入触发器、步骤状态、审批引擎或 Workflow DSL；如果以后确有
   需求，应作为 Skill 可调用的独立编排能力评估，而不是恢复第二套 SOP 内容系统。

## 测试与验收

- 无 `kind` 的现有 Skill 加载结果和摘要保持不变。
- `kind: skill` 与缺省行为等价。
- `kind: sop` 走同一加载和依赖检查路径，并显示 SOP 小标。
- 未知 `kind` 不导致 Skill 消失或 Agent 启动失败。
- 上传、安装、删除、工作区覆盖和安全扫描对两种类型行为一致。
- 中英文 GUI 均不把 SOP 描述成独立系统。
- 不产生 `sops/` 目录或独立 SOP 注册数据。

## 明确不做

- 不建设独立 SOP Engine。
- 不为 SOP Skill 增加隐式自动执行。
- 不因 `kind: sop` 自动提高工具权限或降低审批要求。
- 不把所有含步骤的 Skill 自动识别为 SOP。
- 不立即迁移或重写已有 Skill。

