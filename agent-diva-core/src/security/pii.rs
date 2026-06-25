use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

const DEFAULT_SEVERITY: &str = "warning";

static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}\b").expect("valid email regex")
});
static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)
        (?:
            \+?\d{1,3}[\s.\-]?
        )?
        (?:
            \(\d{3}\)|\d{3}
        )
        [\s.\-]?
        \d{3}
        [\s.\-]?
        \d{4}
    ",
    )
    .expect("valid phone regex")
});
static API_KEY_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?ix)
        \b(
            sk-[a-z0-9._\-]{4,}|
            ghp_[a-z0-9_]{4,}|
            xox[bep]-[a-z0-9._\-]{4,}|
            xoxe-[a-z0-9._\-]{4,}|
            AKIA[0-9A-Z]{16}|
            AIza[0-9A-Za-z\-_]{35}
        )\b
    ",
    )
    .expect("valid api key regex")
});
static CREDIT_CARD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:\d[ -]*?){13,19}\b").expect("valid credit card regex"));
static SSN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("valid ssn regex"));
static URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:https?://|www\.)[^\s<>()]+").expect("valid url regex"));
static IPV4_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b",
    )
    .expect("valid ipv4 regex")
});
static IPV6_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:[a-f0-9]{1,4}:){2,7}[a-f0-9]{1,4}\b").expect("valid ipv6 regex")
});
static NAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)
        (?i:(?:my\ name\ is|i\ am|i'm|this\ is|contact\ person\ is|contact\ is))
        \s+
        (?P<name>[A-Z][a-z]+(?:\s+[A-Z][a-z]+){1,2})
    ",
    )
    .expect("valid name regex")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PiiKind {
    Email,
    Phone,
    ApiKey,
    CreditCard,
    SSN,
    IP,
    URL,
    Name,
}

impl PiiKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Email => "Email",
            Self::Phone => "Phone",
            Self::ApiKey => "ApiKey",
            Self::CreditCard => "CreditCard",
            Self::SSN => "SSN",
            Self::IP => "IP",
            Self::URL => "URL",
            Self::Name => "Name",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiiMatch {
    pub kind: PiiKind,
    pub start: usize,
    pub end: usize,
    pub matched_text: String,
    pub severity: String,
}

impl PiiMatch {
    fn new(kind: PiiKind, text: &str, start: usize, end: usize) -> Self {
        Self {
            kind,
            start,
            end,
            matched_text: text[start..end].to_string(),
            severity: DEFAULT_SEVERITY.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct CandidateMatch {
    order: usize,
    pii: PiiMatch,
}

pub fn redact_pii(text: &str) -> (String, Vec<PiiMatch>) {
    let matches = collect_matches(text);
    if matches.is_empty() {
        return (text.to_string(), Vec::new());
    }

    let mut redacted = String::with_capacity(text.len());
    let mut cursor = 0;
    for pii_match in &matches {
        redacted.push_str(&text[cursor..pii_match.start]);
        redacted.push_str(&format!("[REDACTED:{}]", pii_match.kind.as_str()));
        cursor = pii_match.end;
    }
    redacted.push_str(&text[cursor..]);

    (redacted, matches)
}

fn collect_matches(text: &str) -> Vec<PiiMatch> {
    let mut candidates = Vec::new();

    push_regex_matches(&mut candidates, 0, PiiKind::ApiKey, &API_KEY_RE, text);
    push_regex_matches(
        &mut candidates,
        1,
        PiiKind::CreditCard,
        &CREDIT_CARD_RE,
        text,
    );
    push_regex_matches(&mut candidates, 2, PiiKind::SSN, &SSN_RE, text);
    push_regex_matches(&mut candidates, 3, PiiKind::Email, &EMAIL_RE, text);
    push_regex_matches(&mut candidates, 4, PiiKind::URL, &URL_RE, text);
    push_regex_matches(&mut candidates, 5, PiiKind::IP, &IPV4_RE, text);
    push_regex_matches(&mut candidates, 6, PiiKind::IP, &IPV6_RE, text);
    push_regex_matches(&mut candidates, 7, PiiKind::Phone, &PHONE_RE, text);
    push_name_matches(&mut candidates, 8, text);

    candidates.retain(|candidate| match candidate.pii.kind {
        PiiKind::CreditCard => looks_like_credit_card(&candidate.pii.matched_text),
        _ => true,
    });

    candidates.sort_by(|left, right| {
        left.pii
            .start
            .cmp(&right.pii.start)
            .then(left.order.cmp(&right.order))
            .then((right.pii.end - right.pii.start).cmp(&(left.pii.end - left.pii.start)))
    });

    let mut selected = Vec::new();
    for candidate in candidates {
        if selected
            .last()
            .is_some_and(|last: &PiiMatch| candidate.pii.start < last.end)
        {
            continue;
        }
        selected.push(candidate.pii);
    }
    selected
}

fn push_regex_matches(
    candidates: &mut Vec<CandidateMatch>,
    order: usize,
    kind: PiiKind,
    regex: &Regex,
    text: &str,
) {
    for matched in regex.find_iter(text) {
        candidates.push(CandidateMatch {
            order,
            pii: PiiMatch::new(kind, text, matched.start(), matched.end()),
        });
    }
}

fn push_name_matches(candidates: &mut Vec<CandidateMatch>, order: usize, text: &str) {
    for captures in NAME_RE.captures_iter(text) {
        let Some(name) = captures.name("name") else {
            continue;
        };
        candidates.push(CandidateMatch {
            order,
            pii: PiiMatch::new(PiiKind::Name, text, name.start(), name.end()),
        });
    }
}

fn looks_like_credit_card(candidate: &str) -> bool {
    let digits: String = candidate.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if !(13..=19).contains(&digits.len()) {
        return false;
    }
    luhn_valid(&digits)
}

fn luhn_valid(digits: &str) -> bool {
    let mut sum = 0_u32;
    let mut double = false;

    for ch in digits.chars().rev() {
        let Some(mut digit) = ch.to_digit(10) else {
            return false;
        };
        if double {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        sum += digit;
        double = !double;
    }

    sum % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::{redact_pii, PiiKind};

    #[test]
    fn redacts_email() {
        let (redacted, matches) = redact_pii("Reach me at alice@example.com");
        assert_eq!(redacted, "Reach me at [REDACTED:Email]");
        assert_eq!(matches[0].kind, PiiKind::Email);
        assert_eq!(matches[0].severity, "warning");
    }

    #[test]
    fn redacts_phone() {
        let (redacted, matches) = redact_pii("Call +1 (415) 555-2671 today");
        assert_eq!(redacted, "Call [REDACTED:Phone] today");
        assert_eq!(matches[0].kind, PiiKind::Phone);
    }

    #[test]
    fn redacts_api_key() {
        let (redacted, matches) = redact_pii("token sk-test-secret-value-123456");
        assert_eq!(redacted, "token [REDACTED:ApiKey]");
        assert_eq!(matches[0].kind, PiiKind::ApiKey);
    }

    #[test]
    fn redacts_credit_card() {
        let (redacted, matches) = redact_pii("Visa 4111 1111 1111 1111");
        assert_eq!(redacted, "Visa [REDACTED:CreditCard]");
        assert_eq!(matches[0].kind, PiiKind::CreditCard);
    }

    #[test]
    fn ignores_invalid_credit_card_like_numbers() {
        let (redacted, matches) = redact_pii("Number 4111 1111 1111 1112");
        assert_eq!(redacted, "Number 4111 1111 1111 1112");
        assert!(matches.is_empty());
    }

    #[test]
    fn redacts_ssn() {
        let (redacted, matches) = redact_pii("SSN 123-45-6789");
        assert_eq!(redacted, "SSN [REDACTED:SSN]");
        assert_eq!(matches[0].kind, PiiKind::SSN);
    }

    #[test]
    fn redacts_ip() {
        let (redacted, matches) = redact_pii("Server 10.20.30.40 responded");
        assert_eq!(redacted, "Server [REDACTED:IP] responded");
        assert_eq!(matches[0].kind, PiiKind::IP);
    }

    #[test]
    fn redacts_url() {
        let (redacted, matches) = redact_pii("Open https://example.com/reset?token=abc");
        assert_eq!(redacted, "Open [REDACTED:URL]");
        assert_eq!(matches[0].kind, PiiKind::URL);
    }

    #[test]
    fn redacts_contextual_name() {
        let (redacted, matches) = redact_pii("My name is John Doe.");
        assert_eq!(redacted, "My name is [REDACTED:Name].");
        assert_eq!(matches[0].kind, PiiKind::Name);
    }

    #[test]
    fn redacts_multiple_categories_without_overlap() {
        let (redacted, matches) = redact_pii(
            "Jane Roe email jane@example.com card 4111 1111 1111 1111 site https://example.com",
        );

        assert_eq!(
            redacted,
            "Jane Roe email [REDACTED:Email] card [REDACTED:CreditCard] site [REDACTED:URL]"
        );
        assert_eq!(matches.len(), 3);
    }
}
