---
name: fumadocs-code-dev
description: Help implement and refactor Fumadocs documentation sites (content structure, MDX/Markdown, routing, Rust/TypeScript integration) following project conventions. Use when the user is developing or modifying Fumadocs-based docs or code examples.
---

# Fumadocs 文档代码开发

## 使用场景

在以下场景使用本 Skill：

- 需要搭建或扩展基于 Fumadocs 的文档站点
- 需要为文档编写或重构示例代码（如 Rust、TypeScript、前端组件示例）
- 需要调整文档路由结构、侧边栏、目录或多语言结构
- 需要保证文档中的代码片段与实际项目结构、API 一致

## 开发指引

1. **确认技术栈与目录结构**
   - 优先识别当前 Fumadocs 项目使用的框架（如 Next.js/React 等）
   - 分析 `.workspace` 或仓库中的 Fumadocs 示例/模板目录
   - 避免随意创建与现有模式冲突的新目录结构

2. **内容与路由设计**
   - 先设计文档信息架构（章节、子章节、路由路径）
   - 路由命名保持短、小写、使用连字符（例：`/getting-started`, `/agent-diva/providers`）
   - 对应的文件/目录命名与路由保持一一对应，避免歧义

3. **Markdown / MDX 编写规范**
   - 基础内容使用普通 Markdown，避免过度嵌套组件
   - 代码块使用合适的语言标签（如 `rust`、`ts`、`bash`），避免省略
   - 一篇文档内尽量统一术语（如统一使用 “会话”、“提供商”、“通道”）
   - 长代码示例优先拆分为单独文件并在文档中引用，而不是直接内联超长代码

4. **示例代码与真实代码同步**
   - 若示例代码对应实际仓库中的模块，优先从真实文件中抽取/裁剪
   - 明确标注示例的上下文限制（例如：需要某些特定的 feature 或配置）
   - 避免在文档中出现与当前版本已不一致的 API、类型或配置字段

5. **与 Agent Diva 架构对齐**
   - 解释 Fumadocs 文档与 `agent-diva-*` 各 crate 的关系（core、agent、providers、channels 等）
   - 对外暴露的配置、命令（如 `just` 脚本、CLI 子命令）要有清晰的使用示例
   - 避免在文档示例中引入未说明的额外依赖或复杂脚手架

## 输出格式建议

当用户请求编写或重构 Fumadocs 文档时，优先采用以下结构输出：

```markdown
---
title: 标题
description: 简短描述（1–2 句话）
---

## 场景说明
[简要说明本页解决什么问题]

## 快速开始
[最小可运行示例：命令 + 配置 + 代码片段]

## 详细说明
- 小节 1
- 小节 2

## 参考
- 内部链接或外部链接列表
```

## 示例：为 Agent Diva 编写 Fumadocs 页面

在为 Agent Diva 增加 “Provider 配置” 文档时：

1. 先梳理当前支持的 Provider 及其模型 ID 规则
2. 按照 “避免对原生 Provider 添加 LiteLLM 前缀” 的规则进行说明
3. 提供典型配置片段（JSON / TOML / YAML）与 CLI 使用示例
4. 在文档中加入必要但简短的注意事项，而不是拷贝大量实现细节

在生成内容前，始终优先复用现有约定（如 `AGENTS.md`、`CLAUDE.md` 中的概念与术语）。

