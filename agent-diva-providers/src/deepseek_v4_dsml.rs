//! Strict DeepSeek V4 DSML completion decoding.
//!
//! This module only converts an already received completion. Prompt encoding is
//! deliberately owned by the configured inference gateway.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::base::{ProviderError, ProviderResult, ToolCallRequest};

const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";
const CALLS_OPEN: &str = "<｜DSML｜tool_calls>";
const CALLS_CLOSE: &str = "</｜DSML｜tool_calls>";
const INVOKE_OPEN: &str = "<｜DSML｜invoke";
const INVOKE_CLOSE: &str = "</｜DSML｜invoke>";
const PARAM_OPEN: &str = "<｜DSML｜parameter";
const PARAM_CLOSE: &str = "</｜DSML｜parameter>";

/// A DSML completion converted to the provider-neutral response types.
#[derive(Debug, Clone)]
pub(crate) struct DecodedCompletion {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Vec<ToolCallRequest>,
}

/// Decode V4 reasoning and DSML calls without accepting malformed protocol text.
pub(crate) fn decode_completion(
    completion: &str,
    allowed_tools: &HashSet<String>,
    max_buffer_bytes: usize,
) -> ProviderResult<DecodedCompletion> {
    if completion.len() > max_buffer_bytes {
        return Err(invalid("DSML completion exceeds configured buffer limit"));
    }

    let (reasoning_content, body) = split_thinking(completion)?;
    let Some(calls_start) = body.find(CALLS_OPEN) else {
        return Ok(DecodedCompletion {
            content: non_empty(body),
            reasoning_content,
            tool_calls: Vec::new(),
        });
    };
    let before = &body[..calls_start];
    let after_open = &body[calls_start + CALLS_OPEN.len()..];
    let calls_end = after_open
        .find(CALLS_CLOSE)
        .ok_or_else(|| invalid("DSML tool_calls block is not closed"))?;
    let calls_body = &after_open[..calls_end];
    let after = &after_open[calls_end + CALLS_CLOSE.len()..];
    if after.contains(CALLS_OPEN) || calls_body.contains(CALLS_OPEN) {
        return Err(invalid(
            "DSML completion contains multiple tool_calls blocks",
        ));
    }

    let tool_calls = parse_calls(calls_body, allowed_tools)?;
    let visible = format!("{before}{after}");
    Ok(DecodedCompletion {
        content: non_empty(&visible),
        reasoning_content,
        tool_calls,
    })
}

fn split_thinking(completion: &str) -> ProviderResult<(Option<String>, &str)> {
    let Some(start) = completion.find(THINK_OPEN) else {
        if completion.contains(THINK_CLOSE) {
            return Err(invalid("DSML completion has an unexpected </think> tag"));
        }
        return Ok((None, completion));
    };
    if !completion[..start].trim().is_empty() {
        return Err(invalid(
            "DSML thinking block must precede completion content",
        ));
    }
    let remaining = &completion[start + THINK_OPEN.len()..];
    let end = remaining
        .find(THINK_CLOSE)
        .ok_or_else(|| invalid("DSML thinking block is not closed"))?;
    if remaining[end + THINK_CLOSE.len()..].contains(THINK_OPEN) {
        return Err(invalid("DSML completion contains multiple thinking blocks"));
    }
    Ok((
        non_empty(&remaining[..end]),
        &remaining[end + THINK_CLOSE.len()..],
    ))
}

fn parse_calls(
    body: &str,
    allowed_tools: &HashSet<String>,
) -> ProviderResult<Vec<ToolCallRequest>> {
    let mut rest = body;
    let mut calls = Vec::new();
    while !rest.trim().is_empty() {
        let start = rest
            .find(INVOKE_OPEN)
            .ok_or_else(|| invalid("DSML tool_calls contains unexpected content"))?;
        if !rest[..start].trim().is_empty() {
            return Err(invalid("DSML tool_calls contains unexpected text"));
        }
        let (attributes, after_tag) = take_start_tag(&rest[start..], INVOKE_OPEN)?;
        let name = required_attr(&attributes, "name")?;
        if !allowed_tools.contains(name) {
            return Err(invalid(format!("DSML invoked unknown tool '{name}'")));
        }
        let end = after_tag
            .find(INVOKE_CLOSE)
            .ok_or_else(|| invalid("DSML invoke block is not closed"))?;
        let invoke_body = &after_tag[..end];
        if invoke_body.contains(INVOKE_OPEN) {
            return Err(invalid("DSML nested invoke blocks are not allowed"));
        }
        calls.push(ToolCallRequest {
            id: format!("deepseek_v4_dsml_{}", calls.len()),
            call_type: "function".to_string(),
            name: name.to_string(),
            arguments: parse_parameters(invoke_body)?,
        });
        rest = &after_tag[end + INVOKE_CLOSE.len()..];
    }
    if calls.is_empty() {
        return Err(invalid("DSML tool_calls block is empty"));
    }
    Ok(calls)
}

fn parse_parameters(body: &str) -> ProviderResult<HashMap<String, Value>> {
    let mut rest = body;
    let mut parameters = HashMap::new();
    while !rest.trim().is_empty() {
        let start = rest
            .find(PARAM_OPEN)
            .ok_or_else(|| invalid("DSML invoke contains unexpected content"))?;
        if !rest[..start].trim().is_empty() {
            return Err(invalid("DSML invoke contains unexpected text"));
        }
        let (attributes, after_tag) = take_start_tag(&rest[start..], PARAM_OPEN)?;
        let name = required_attr(&attributes, "name")?;
        let string_value = required_attr(&attributes, "string")?;
        let end = after_tag
            .find(PARAM_CLOSE)
            .ok_or_else(|| invalid("DSML parameter block is not closed"))?;
        let raw_value = &after_tag[..end];
        let value = match string_value {
            "true" => Value::String(raw_value.to_string()),
            "false" => serde_json::from_str(raw_value).map_err(|error| {
                invalid(format!(
                    "DSML parameter '{name}' is not valid JSON: {error}"
                ))
            })?,
            _ => {
                return Err(invalid(
                    "DSML parameter string attribute must be true or false",
                ))
            }
        };
        if parameters.insert(name.to_string(), value).is_some() {
            return Err(invalid(format!("DSML parameter '{name}' is duplicated")));
        }
        rest = &after_tag[end + PARAM_CLOSE.len()..];
    }
    Ok(parameters)
}

fn take_start_tag<'a>(
    input: &'a str,
    prefix: &str,
) -> ProviderResult<(HashMap<String, String>, &'a str)> {
    let end = input
        .find('>')
        .ok_or_else(|| invalid("DSML start tag is not closed"))?;
    let tag = &input[..end];
    let attrs = tag
        .strip_prefix(prefix)
        .ok_or_else(|| invalid("DSML tag prefix is invalid"))?;
    Ok((parse_attributes(attrs)?, &input[end + 1..]))
}

fn parse_attributes(input: &str) -> ProviderResult<HashMap<String, String>> {
    let mut attrs = HashMap::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        let equals = rest
            .find('=')
            .ok_or_else(|| invalid("DSML attribute has no value"))?;
        let key = rest[..equals].trim();
        if key.is_empty() || key.chars().any(char::is_whitespace) {
            return Err(invalid("DSML attribute name is invalid"));
        }
        let value_start = rest[equals + 1..].trim_start();
        let value = value_start
            .strip_prefix('"')
            .ok_or_else(|| invalid("DSML attribute must use double quotes"))?;
        let quote = value
            .find('"')
            .ok_or_else(|| invalid("DSML attribute quote is not closed"))?;
        if attrs
            .insert(key.to_string(), value[..quote].to_string())
            .is_some()
        {
            return Err(invalid(format!("DSML attribute '{key}' is duplicated")));
        }
        rest = value[quote + 1..].trim_start();
    }
    Ok(attrs)
}

fn required_attr<'a>(attrs: &'a HashMap<String, String>, name: &str) -> ProviderResult<&'a str> {
    attrs
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(format!("DSML tag is missing '{name}' attribute")))
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_string())
}

fn invalid(message: impl Into<String>) -> ProviderError {
    ProviderError::InvalidResponse(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> HashSet<String> {
        ["write_file".to_string(), "read_file".to_string()]
            .into_iter()
            .collect()
    }

    #[test]
    fn decodes_thinking_and_typed_parameters() {
        let decoded = decode_completion(
            "<think>plan</think><｜DSML｜tool_calls><｜DSML｜invoke name=\"write_file\"><｜DSML｜parameter name=\"path\" string=\"true\">a.txt</｜DSML｜parameter><｜DSML｜parameter name=\"payload\" string=\"false\">{\"ok\":true}</｜DSML｜parameter></｜DSML｜invoke></｜DSML｜tool_calls>",
            &tools(),
            4096,
        )
        .unwrap();
        assert_eq!(decoded.reasoning_content.as_deref(), Some("plan"));
        assert_eq!(decoded.tool_calls[0].arguments["path"], "a.txt");
        assert_eq!(decoded.tool_calls[0].arguments["payload"]["ok"], true);
    }

    #[test]
    fn rejects_malformed_or_unknown_calls() {
        let malformed = "<｜DSML｜tool_calls><｜DSML｜invoke name=\"unknown\"></｜DSML｜invoke></｜DSML｜tool_calls>";
        assert!(decode_completion(malformed, &tools(), 4096).is_err());
        let duplicate = "<｜DSML｜tool_calls><｜DSML｜invoke name=\"read_file\"><｜DSML｜parameter name=\"x\" string=\"true\">1</｜DSML｜parameter><｜DSML｜parameter name=\"x\" string=\"true\">2</｜DSML｜parameter></｜DSML｜invoke></｜DSML｜tool_calls>";
        assert!(decode_completion(duplicate, &tools(), 4096).is_err());
    }
}
