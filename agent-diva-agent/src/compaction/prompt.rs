//! Compaction system prompt — instructs the LLM to produce a dense,
//! lossy summary of the conversation that preserves all actionable context.

/// System prompt used when calling the LLM for context compaction.
///
/// The prompt requires a structured `<analysis>` / `<summary>` output
/// so the compactor can extract the summary portion deterministically.
pub const COMPACTION_SYSTEM_PROMPT: &str = r#"你是一个对话压缩器。你的任务是将以下对话压缩为一份密集、有损的摘要，保留所有可执行上下文。

请严格按照以下结构输出：

<analysis>
（简要分析对话的关键主题、决策、操作和当前状态。用第三人称过去时。）
</analysis>

<summary>
（压缩后的摘要。必须保留以下信息：
- 项目状态、活跃任务、已做出的决策
- 用户偏好、身份、约束条件
- 工具调用：做了什么、为什么做
- 编辑过的文件路径、执行过的命令、产生的结果
- 待解决问题、阻塞项、下一步计划
用第三人称过去时书写。信息密度高。最多 2000 字符。）
</summary>

重要规则：
- 只输出上述结构，不要有任何前言、后记或元评论
- 不要编造对话中不存在的信息
- 对不确定的内容标注 [不确定]
- 用中文撰写摘要（对话原文若是英文，保留关键术语）
"#;
