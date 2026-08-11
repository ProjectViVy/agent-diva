//! Versioned LLM-facing prompt contract for the canonical checkpoint.

pub const CHECKPOINT_PROMPT_ID: &str = "canonical_checkpoint_v1";
pub const CHECKPOINT_PROMPT_VERSION: u16 = 1;

pub const CHECKPOINT_SYSTEM_PROMPT: &str = r#"You maintain one canonical checkpoint for a long-running conversation.
Return only a <checkpoint>...</checkpoint> body with exactly these sections:

## 目标与约束
## 已完成事项
## 关键决定
## 当前状态
## 未解决问题
## 下一步
## 保留标识符与 artifact 引用

Merge the previous checkpoint with the newly supplied compacted prefix. Preserve
paths, IDs, commands, decisions, blockers, and artifact references. Do not
invent facts. Mark uncertainty as [uncertain]. Completed tool groups are already
mechanically represented; do not reproduce their large result bodies."#;

pub fn checkpoint_request(
    previous_body: Option<&str>,
    message_count: usize,
    formatted: &str,
) -> String {
    let previous = previous_body.unwrap_or("（无旧检查点；从本次输入建立。）");
    format!(
        "旧 canonical checkpoint（仅一份，成功后替换）：\n<previous_checkpoint>\n{previous}\n</previous_checkpoint>\n\n本次安全裁剪的 {message_count} 条消息（工具链已机械折叠）：\n\n{formatted}\n\n请输出新的完整 canonical checkpoint。"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contract_is_versioned_and_structured() {
        assert_eq!(CHECKPOINT_PROMPT_ID, "canonical_checkpoint_v1");
        assert_eq!(CHECKPOINT_PROMPT_VERSION, 1);
        assert!(CHECKPOINT_SYSTEM_PROMPT.contains("<checkpoint>"));
    }
}
