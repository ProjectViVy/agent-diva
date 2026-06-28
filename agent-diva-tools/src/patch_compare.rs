pub(crate) fn trimmed_lines_equal(candidate: &str, expected: &str) -> bool {
    trimmed_lines(candidate) == trimmed_lines(expected)
}

pub(crate) fn normalized_whitespace_equal(candidate: &str, expected: &str) -> bool {
    normalize_whitespace(candidate) == normalize_whitespace(expected)
}

pub(crate) fn case_insensitive_equal(candidate: &str, expected: &str) -> bool {
    candidate
        .trim_end_matches('\n')
        .eq_ignore_ascii_case(expected)
}

pub(crate) fn indent_tolerant_equal(candidate: &str, expected: &str) -> bool {
    trim_start_lines(candidate) == trim_start_lines(expected)
}

pub(crate) fn fuzzy_equal(candidate: &str, expected: &str) -> bool {
    let left = normalize_whitespace(candidate).to_ascii_lowercase();
    let right = normalize_whitespace(expected).to_ascii_lowercase();
    if left.is_empty() || right.is_empty() {
        return false;
    }

    let distance = levenshtein(&left, &right);
    let max_len = left.len().max(right.len());
    distance <= fuzzy_threshold(max_len)
}

fn fuzzy_threshold(max_len: usize) -> usize {
    if max_len <= 8 {
        1
    } else if max_len <= 24 {
        2
    } else {
        (max_len / 8).max(3)
    }
}

fn trimmed_lines(text: &str) -> Vec<String> {
    text.lines().map(|line| line.trim().to_string()).collect()
}

fn trim_start_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim_start().to_string())
        .collect()
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn levenshtein(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_ch) in left_chars.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_ch) in right_chars.iter().enumerate() {
            let cost = usize::from(left_ch != right_ch);
            current[right_index + 1] = (current[right_index] + 1)
                .min(previous[right_index + 1] + 1)
                .min(previous[right_index] + cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}
