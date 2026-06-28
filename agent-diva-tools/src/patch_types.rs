use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) enum PatchMatchStrategy {
    Exact,
    TrimWhitespace,
    NormalizeWhitespace,
    CaseInsensitive,
    IndentTolerance,
    Fuzzy,
    LineBased,
    PartialMatch,
    Regex,
}

impl PatchMatchStrategy {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "Exact",
            Self::TrimWhitespace => "TrimWhitespace",
            Self::NormalizeWhitespace => "NormalizeWhitespace",
            Self::CaseInsensitive => "CaseInsensitive",
            Self::IndentTolerance => "IndentTolerance",
            Self::Fuzzy => "Fuzzy",
            Self::LineBased => "LineBased",
            Self::PartialMatch => "PartialMatch",
            Self::Regex => "Regex",
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatchRequest {
    pub(crate) path: String,
    pub(crate) old_text: String,
    pub(crate) new_text: String,
    #[serde(default = "default_match_strategy")]
    pub(crate) match_strategy: PatchMatchStrategy,
}

fn default_match_strategy() -> PatchMatchStrategy {
    PatchMatchStrategy::Exact
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpanMatch {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) matched_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchSuccess {
    pub(crate) strategy: PatchMatchStrategy,
    pub(crate) matched_text: String,
    pub(crate) replacement_text: String,
    pub(crate) new_content: String,
}
