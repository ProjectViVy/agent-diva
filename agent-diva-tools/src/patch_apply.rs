use super::patch_compare::{
    case_insensitive_equal, fuzzy_equal, indent_tolerant_equal, normalized_whitespace_equal,
    trimmed_lines_equal,
};
use super::patch_types::{PatchMatchStrategy, PatchRequest, PatchSuccess, SpanMatch};
use regex::RegexBuilder;

pub(crate) fn apply_patch(
    content: &str,
    request: &PatchRequest,
) -> std::result::Result<PatchSuccess, String> {
    let matched_span = match request.match_strategy {
        PatchMatchStrategy::Exact => find_exact_match(content, &request.old_text),
        PatchMatchStrategy::TrimWhitespace => {
            find_line_strategy_match(content, &request.old_text, trimmed_lines_equal)
        }
        PatchMatchStrategy::NormalizeWhitespace => {
            find_line_strategy_match(content, &request.old_text, normalized_whitespace_equal)
        }
        PatchMatchStrategy::CaseInsensitive => {
            find_line_strategy_match(content, &request.old_text, case_insensitive_equal)
        }
        PatchMatchStrategy::IndentTolerance => {
            find_line_strategy_match(content, &request.old_text, indent_tolerant_equal)
        }
        PatchMatchStrategy::Fuzzy => {
            find_line_strategy_match(content, &request.old_text, fuzzy_equal)
        }
        PatchMatchStrategy::LineBased => find_line_based_match(content, &request.old_text),
        PatchMatchStrategy::PartialMatch => find_partial_match(content, &request.old_text),
        PatchMatchStrategy::Regex => find_regex_match(content, &request.old_text),
    }?;

    let replacement_text = materialize_replacement_text(content, request, &matched_span);
    let mut new_content = String::with_capacity(
        content.len()
            + replacement_text
                .len()
                .saturating_sub(matched_span.matched_text.len()),
    );
    new_content.push_str(&content[..matched_span.start]);
    new_content.push_str(&replacement_text);
    new_content.push_str(&content[matched_span.end..]);

    Ok(PatchSuccess {
        strategy: request.match_strategy.clone(),
        matched_text: matched_span.matched_text,
        replacement_text,
        new_content,
    })
}

fn materialize_replacement_text(
    content: &str,
    request: &PatchRequest,
    matched_span: &SpanMatch,
) -> String {
    let mut replacement_text = request.new_text.clone();
    if preserves_trailing_newline(&request.match_strategy)
        && matched_span.matched_text.ends_with('\n')
        && !replacement_text.ends_with('\n')
        && (matched_span.end < content.len() || content.ends_with('\n'))
    {
        replacement_text.push('\n');
    }
    replacement_text
}

fn preserves_trailing_newline(strategy: &PatchMatchStrategy) -> bool {
    matches!(
        strategy,
        PatchMatchStrategy::TrimWhitespace
            | PatchMatchStrategy::NormalizeWhitespace
            | PatchMatchStrategy::IndentTolerance
            | PatchMatchStrategy::Fuzzy
            | PatchMatchStrategy::LineBased
            | PatchMatchStrategy::PartialMatch
    )
}

fn find_exact_match(content: &str, old_text: &str) -> std::result::Result<SpanMatch, String> {
    let matches = content.match_indices(old_text).collect::<Vec<_>>();
    if matches.is_empty() {
        return Err("Exact match did not find old_text in the file".to_string());
    }
    if matches.len() > 1 {
        return Err(format!(
            "Exact match found {} locations; provide more context or choose a stricter strategy",
            matches.len()
        ));
    }

    let (start, matched) = matches[0];
    Ok(SpanMatch {
        start,
        end: start + matched.len(),
        matched_text: matched.to_string(),
    })
}

fn find_regex_match(content: &str, pattern: &str) -> std::result::Result<SpanMatch, String> {
    let regex = RegexBuilder::new(pattern)
        .multi_line(true)
        .build()
        .map_err(|error| format!("Invalid regex pattern: {}", error))?;
    let matches = regex.find_iter(content).collect::<Vec<_>>();
    if matches.is_empty() {
        return Err("Regex pattern did not match the file".to_string());
    }
    if matches.len() > 1 {
        return Err(format!(
            "Regex pattern matched {} locations; refine the pattern to a unique target",
            matches.len()
        ));
    }

    let only = matches[0];
    Ok(SpanMatch {
        start: only.start(),
        end: only.end(),
        matched_text: only.as_str().to_string(),
    })
}

#[derive(Debug, Clone, Copy)]
struct LineSpan {
    start: usize,
    end: usize,
}

fn compute_line_spans(content: &str) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut start = 0;
    for (index, ch) in content.char_indices() {
        if ch == '\n' {
            spans.push(LineSpan {
                start,
                end: index + ch.len_utf8(),
            });
            start = index + ch.len_utf8();
        }
    }
    if start < content.len() {
        spans.push(LineSpan {
            start,
            end: content.len(),
        });
    }
    if spans.is_empty() {
        spans.push(LineSpan { start: 0, end: 0 });
    }
    spans
}

fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.split('\n').count()
    }
}

fn find_line_strategy_match(
    content: &str,
    old_text: &str,
    comparator: fn(&str, &str) -> bool,
) -> std::result::Result<SpanMatch, String> {
    let spans = compute_line_spans(content);
    let line_count = count_lines(old_text);
    if line_count == 0 {
        return Err("old_text must contain at least one line".to_string());
    }
    if line_count > spans.len() {
        return Err("Patch target does not exist in the file".to_string());
    }

    let mut matches = Vec::new();
    for start_index in 0..=(spans.len() - line_count) {
        let start = spans[start_index].start;
        let end = spans[start_index + line_count - 1].end;
        let candidate = &content[start..end];
        if comparator(candidate, old_text) {
            matches.push(SpanMatch {
                start,
                end,
                matched_text: candidate.to_string(),
            });
        }
    }

    single_match(matches)
}

fn find_line_based_match(content: &str, old_text: &str) -> std::result::Result<SpanMatch, String> {
    let old_trimmed = old_text.trim_end_matches('\n');
    find_line_strategy_match(content, old_trimmed, |candidate, expected| {
        candidate.trim_end_matches('\n') == expected
    })
}

fn find_partial_match(content: &str, old_text: &str) -> std::result::Result<SpanMatch, String> {
    let spans = compute_line_spans(content);
    let line_count = count_lines(old_text).max(1);
    let target = old_text.trim();
    let mut matches = Vec::new();
    for window_len in 1..=line_count {
        if window_len > spans.len() {
            break;
        }
        for start_index in 0..=(spans.len() - window_len) {
            let start = spans[start_index].start;
            let end = spans[start_index + window_len - 1].end;
            let candidate = &content[start..end];
            if candidate.trim().contains(target) {
                matches.push(SpanMatch {
                    start,
                    end,
                    matched_text: candidate.to_string(),
                });
            }
        }
    }
    single_match(matches)
}

fn single_match(matches: Vec<SpanMatch>) -> std::result::Result<SpanMatch, String> {
    if matches.is_empty() {
        return Err("Patch strategy could not find a unique match".to_string());
    }

    let unique = dedupe_matches(matches);
    if unique.len() > 1 {
        return Err(format!(
            "Patch strategy found {} candidate locations; provide more context or switch strategy",
            unique.len()
        ));
    }

    Ok(unique.into_iter().next().expect("checked non-empty"))
}

fn dedupe_matches(matches: Vec<SpanMatch>) -> Vec<SpanMatch> {
    let mut unique = Vec::new();
    for item in matches {
        if !unique
            .iter()
            .any(|candidate: &SpanMatch| candidate.start == item.start && candidate.end == item.end)
        {
            unique.push(item);
        }
    }
    unique
}
