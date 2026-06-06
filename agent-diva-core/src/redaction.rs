use once_cell::sync::Lazy;
use regex::{Captures, Regex};

const REDACTION: &str = "***REDACTED***";

static BEARER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bBearer\s+([A-Za-z0-9._\-]+)").expect("valid bearer regex"));
static PREFIX_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:sk-[A-Za-z0-9._\-]+|ghp_[A-Za-z0-9_]+|xoxb-[A-Za-z0-9._\-]+|xoxe-[A-Za-z0-9._\-]+|xoxp-[A-Za-z0-9._\-]+)\b")
        .expect("valid prefix token regex")
});
static JSON_FIELD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?ix)
        (?P<key>"(?:api_key|token|secret|password|authorization)")
        \s*:\s*
        (?P<value>"[^"]*"|null)
    "#,
    )
    .expect("valid json field regex")
});
static DEBUG_SOME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?ix)
        (?P<key>\b(?:api_key|token|secret|password|authorization)\b)
        \s*:\s*
        Some\(".*?"\)
    "#,
    )
    .expect("valid debug some regex")
});
static DEBUG_STRING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?ix)
        (?P<key>\b(?:api_key|token|secret|password|authorization)\b)
        \s*:\s*
        ".*?"
    "#,
    )
    .expect("valid debug string regex")
});
static DEBUG_BARE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?ix)
        (?P<key>\b(?:api_key|token|secret|password|authorization)\b)
        \s*[:=]\s*
        (?P<value>[^\s,}]+)
    "#,
    )
    .expect("valid bare debug regex")
});

pub fn redact_secrets(input: &str) -> String {
    let after_json = JSON_FIELD_RE.replace_all(input, |caps: &Captures| {
        format!(r#"{}: "{}""#, &caps["key"], REDACTION)
    });
    let after_debug_some = DEBUG_SOME_RE.replace_all(&after_json, |caps: &Captures| {
        format!(r#"{}: Some("{}")"#, &caps["key"], REDACTION)
    });
    let after_debug_string = DEBUG_STRING_RE.replace_all(&after_debug_some, |caps: &Captures| {
        format!(r#"{}: "{}""#, &caps["key"], REDACTION)
    });
    let after_debug_bare = DEBUG_BARE_RE.replace_all(&after_debug_string, |caps: &Captures| {
        format!(r#"{}: {}"#, &caps["key"], REDACTION)
    });
    let after_bearer = BEARER_RE.replace_all(&after_debug_bare, format!("Bearer {REDACTION}"));
    PREFIX_TOKEN_RE
        .replace_all(&after_bearer, REDACTION)
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::redact_secrets;

    #[test]
    fn redacts_bearer_token() {
        let redacted = redact_secrets("Authorization: Bearer sk-secret-token");
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("sk-secret-token"));
    }

    #[test]
    fn redacts_json_fields() {
        let redacted = redact_secrets(r#"{"api_key":"sk-test","token":"ghp_demo"}"#);
        assert!(redacted.contains(r#""api_key": "***REDACTED***""#));
        assert!(redacted.contains(r#""token": "***REDACTED***""#));
        assert!(!redacted.contains("sk-test"));
        assert!(!redacted.contains("ghp_demo"));
    }

    #[test]
    fn redacts_debug_output() {
        let redacted = redact_secrets(
            r#"ConfigUpdate { api_key: Some("sk-test"), authorization: "Bearer sk-test" }"#,
        );
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("sk-test"));
    }

    #[test]
    fn redacts_prefix_tokens_inside_text() {
        let redacted = redact_secrets("tokens: ghp_demo and xoxb-secret");
        assert_eq!(redacted, "tokens: ***REDACTED*** and ***REDACTED***");
    }
}
