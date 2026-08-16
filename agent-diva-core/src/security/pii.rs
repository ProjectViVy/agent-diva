//! PII detection and redaction module.
//!
//! Detects and redacts personally identifiable information across 8 categories:
//! email, phone, Chinese ID, credit card, API keys, IP addresses, passport,
//! and bank card numbers.
//!
//! # Severity levels
//!
//! - **Warning**: PII is replaced with `[REDACTED:Kind]`, processing continues.
//! - **Error**: Message is rejected with `SecurityError::PiiDetected`.
//!
//! # Example
//!
//! ```rust,ignore
//! use agent_diva_core::security::pii::{redact_pii, PiiConfig};
//!
//! let config = PiiConfig::default();
//! let result = redact_pii("Contact me at user@example.com", &config);
//! assert!(result.redacted.contains("[REDACTED:Email]"));
//! ```

use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::audit::{self, AuditEvent, PiiSeverity};

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// Categories of personally identifiable information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum PiiKind {
    Email,
    Phone,
    ChineseId,
    CreditCard,
    ApiKey,
    IpAddress,
    Passport,
    BankCard,
    Custom(String),
}

impl PiiKind {
    fn label(&self) -> &str {
        match self {
            Self::Email => "Email",
            Self::Phone => "Phone",
            Self::ChineseId => "ChineseId",
            Self::CreditCard => "CreditCard",
            Self::ApiKey => "ApiKey",
            Self::IpAddress => "IpAddress",
            Self::Passport => "Passport",
            Self::BankCard => "BankCard",
            Self::Custom(name) => name,
        }
    }
}

impl std::fmt::Display for PiiKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A single PII match within input text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    pub kind: PiiKind,
    pub value: String,
    pub start: usize,
    pub end: usize,
    pub severity: PiiSeverity,
}

/// Result of PII redaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionResult {
    pub redacted: String,
    pub detected: Vec<PiiMatch>,
    pub max_severity: Option<PiiSeverity>,
}

/// Per-category severity overrides and default severity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PiiConfig {
    /// Default severity when no category-specific override is set.
    pub default_severity: PiiSeverity,
    /// Enable PII detection entirely.
    pub enabled: bool,
    /// Per-category severity overrides (key = PiiKind label).
    pub category_severity: std::collections::HashMap<String, PiiSeverity>,
}

impl Default for PiiConfig {
    fn default() -> Self {
        Self {
            default_severity: PiiSeverity::Warning,
            // Runtime paths must not hide phones, ids, cards, or similar
            // format matches. Detection stays available when explicitly enabled.
            enabled: false,
            category_severity: std::collections::HashMap::new(),
        }
    }
}

impl PiiConfig {
    /// Resolve the effective severity for a PII category.
    pub fn severity_for(&self, kind: &PiiKind) -> PiiSeverity {
        self.category_severity
            .get(kind.label())
            .copied()
            .unwrap_or(self.default_severity)
    }
}

/// Error type for PII module operations.
#[derive(Debug, thiserror::Error)]
pub enum PiiError {
    #[error("invalid regex pattern for custom PII rule '{name}': {source}")]
    InvalidPattern { name: String, source: regex::Error },
}

// ---------------------------------------------------------------------------
// Custom rule storage (Laputa stub)
// ---------------------------------------------------------------------------

struct CustomPiiRule {
    name: String,
    pattern: Regex,
    #[allow(dead_code)]
    severity: PiiSeverity,
}

static CUSTOM_RULES: OnceLock<parking_lot::RwLock<Vec<CustomPiiRule>>> = OnceLock::new();

fn custom_rules() -> &'static parking_lot::RwLock<Vec<CustomPiiRule>> {
    CUSTOM_RULES.get_or_init(|| parking_lot::RwLock::new(Vec::new()))
}

/// Add a custom PII rule (Laputa integration stub).
///
/// The rule is stored in a global list and will be evaluated by
/// `redact_pii`. This is a stub — no Laputa wiring is implemented.
pub fn add_custom_rule(name: &str, pattern: &str, severity: PiiSeverity) -> Result<(), PiiError> {
    let re = Regex::new(pattern).map_err(|source| PiiError::InvalidPattern {
        name: name.to_string(),
        source,
    })?;
    custom_rules().write().push(CustomPiiRule {
        name: name.to_string(),
        pattern: re,
        severity,
    });
    Ok(())
}

// ---------------------------------------------------------------------------
// Compiled regex patterns
// ---------------------------------------------------------------------------

struct PiiPatterns {
    email: Regex,
    phone_cn: Regex,
    phone_intl: Regex,
    chinese_id: Regex,
    credit_card: Regex,
    api_sk: Regex,
    api_pk: Regex,
    api_ghp: Regex,
    api_gho: Regex,
    api_github_pat: Regex,
    api_bearer: Regex,
    ipv4: Regex,
    ipv6: Regex,
    passport_cn: Regex,
    passport_general: Regex,
    bank_card: Regex,
    version_like: Regex,
    /// Stable BML / checkpoint identifiers. Their digit runs look like phones
    /// or cards and must stay intact so agents can update or remove records.
    memory_record_id: Regex,
}

static PATTERNS: OnceLock<PiiPatterns> = OnceLock::new();

fn patterns() -> &'static PiiPatterns {
    PATTERNS.get_or_init(|| PiiPatterns {
        email: Regex::new(r"[\w._%+-]+@[\w.-]+\.[A-Za-z]{2,}").unwrap(),
        phone_cn: Regex::new(r"\b1[3-9]\d{9}\b").unwrap(),
        phone_intl: Regex::new(r"\b\+?\d{7,15}\b").unwrap(),
        chinese_id: Regex::new(r"\b[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b").unwrap(),
        credit_card: Regex::new(r"\b(?:\d[ -]*?){13,19}\b").unwrap(),
        api_sk: Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap(),
        api_pk: Regex::new(r"pk-[A-Za-z0-9]{20,}").unwrap(),
        api_ghp: Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap(),
        api_gho: Regex::new(r"gho_[A-Za-z0-9]{36}").unwrap(),
        api_github_pat: Regex::new(r"github_pat_[A-Za-z0-9_]{82}").unwrap(),
        api_bearer: Regex::new(r"Bearer [A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_=]+").unwrap(),
        ipv4: Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b").unwrap(),
        ipv6: Regex::new(r"(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(?:[0-9a-fA-F]{1,4}:){1,7}:|(?:[0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|(?:[0-9a-fA-F]{1,4}:){1,5}(?::[0-9a-fA-F]{1,4}){1,2}|(?:[0-9a-fA-F]{1,4}:){1,4}(?::[0-9a-fA-F]{1,4}){1,3}|(?:[0-9a-fA-F]{1,4}:){1,3}(?::[0-9a-fA-F]{1,4}){1,4}|(?:[0-9a-fA-F]{1,4}:){1,2}(?::[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:(?::[0-9a-fA-F]{1,4}){1,6}|:(?::[0-9a-fA-F]{1,4}){1,7}|::(?:[fF]{4}:)?(?:\d{1,3}\.){3}\d{1,3}|(?:[0-9a-fA-F]{1,4}:){1,4}:(?:\d{1,3}\.){3}\d{1,3}").unwrap(),
        passport_cn: Regex::new(r"\bE\d{8}\b").unwrap(),
        passport_general: Regex::new(r"\b[A-Z]{1,2}\d{6,9}\b").unwrap(),
        bank_card: Regex::new(r"\b\d{16,19}\b").unwrap(),
        version_like: Regex::new(r"\bversion\s+\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
        memory_record_id: Regex::new(
            r"\b(?:memory-(?:\d{10,20}-[0-9a-fA-F]{8,64}|tombstone-\d{10,20}|(?:legacy|laputa)-[0-9a-fA-F]{8,64})|session-checkpoint-[0-9a-fA-F]{8,64})\b",
        )
        .unwrap(),
    })
}

fn overlaps_span(start: usize, end: usize, spans: &[(usize, usize)]) -> bool {
    spans
        .iter()
        .any(|&(span_start, span_end)| start < span_end && end > span_start)
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Luhn algorithm validation for credit card / bank card numbers.
fn luhn_check(digits: &str) -> bool {
    let digits_only: String = digits.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits_only.len() < 2 {
        return false;
    }
    // All-zero digits is not a valid card number
    if digits_only.chars().all(|c| c == '0') {
        return false;
    }
    let mut sum: u32 = 0;
    let mut alternate = false;
    for ch in digits_only.chars().rev() {
        let mut d = ch.to_digit(10).unwrap_or(0);
        if alternate {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
        alternate = !alternate;
    }
    sum % 10 == 0
}

/// Chinese national ID (18-digit) checksum validation.
///
/// Uses the weighted-sum mod-11 algorithm with 'X' check digit.
fn chinese_id_checksum(id: &str) -> bool {
    let id = id.trim().to_uppercase();
    let bytes = id.as_bytes();
    if bytes.len() != 18 {
        return false;
    }
    let weights: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    let check_chars: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

    let mut sum: u32 = 0;
    for i in 0..17 {
        let d = if bytes[i].is_ascii_digit() {
            (bytes[i] - b'0') as u32
        } else {
            return false;
        };
        sum += d * weights[i];
    }
    let expected = check_chars[(sum % 11) as usize];
    let actual = bytes[17] as char;
    expected == actual
}

/// Validate that IPv4 octets are in 0..=255 range.
fn valid_ipv4(o1: &str, o2: &str, o3: &str, o4: &str) -> bool {
    [o1, o2, o3, o4].iter().all(|o| o.parse::<u8>().is_ok())
}

// ---------------------------------------------------------------------------
// Core redaction logic
// ---------------------------------------------------------------------------

/// Candidate match before severity resolution.
struct Candidate {
    kind: PiiKind,
    start: usize,
    end: usize,
    value: String,
    skip: bool, // mark for removal (e.g. false positive)
}

/// Detect PII in the input string and return a redacted version.
///
/// - Warning severity: replace PII with `[REDACTED:Kind]`
/// - Error severity: return early with the detected items (caller should reject)
pub fn redact_pii(input: &str, config: &PiiConfig) -> RedactionResult {
    if !config.enabled {
        return RedactionResult {
            redacted: input.to_string(),
            detected: Vec::new(),
            max_severity: None,
        };
    }

    let p = patterns();
    let mut candidates: Vec<Candidate> = Vec::new();

    // Version-like patterns to exclude from IPv4 matches
    let version_spans: Vec<(usize, usize)> = p
        .version_like
        .find_iter(input)
        .map(|m| (m.start(), m.end()))
        .collect();

    // Memory record / checkpoint ids embed long digit runs. Those runs must
    // stay intact: agents use the raw id for update and remove.
    let protected_id_spans: Vec<(usize, usize)> = p
        .memory_record_id
        .find_iter(input)
        .map(|m| (m.start(), m.end()))
        .collect();

    // 1. Email
    for m in p.email.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::Email,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }

    // 2. Phone (Chinese mobile first, then international)
    for m in p.phone_cn.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::Phone,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }
    for m in p.phone_intl.find_iter(input) {
        // Skip if already covered by Chinese phone
        let overlaps = candidates.iter().any(|c| {
            !c.skip && c.kind == PiiKind::Phone && c.start <= m.start() && c.end >= m.end()
        });
        if !overlaps {
            candidates.push(Candidate {
                kind: PiiKind::Phone,
                start: m.start(),
                end: m.end(),
                value: m.as_str().to_string(),
                skip: false,
            });
        }
    }

    // 3. Chinese ID (with checksum)
    for m in p.chinese_id.find_iter(input) {
        let valid = chinese_id_checksum(m.as_str());
        candidates.push(Candidate {
            kind: PiiKind::ChineseId,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: !valid, // invalid checksum → skip
        });
    }

    // 4. Credit card (with Luhn)
    for m in p.credit_card.find_iter(input) {
        let digits: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 13 || digits.len() > 19 {
            continue;
        }
        if luhn_check(&digits) {
            candidates.push(Candidate {
                kind: PiiKind::CreditCard,
                start: m.start(),
                end: m.end(),
                value: m.as_str().to_string(),
                skip: false,
            });
        }
    }

    // 5. API keys
    for m in p.api_sk.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::ApiKey,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }
    for m in p.api_pk.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::ApiKey,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }
    for m in p.api_ghp.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::ApiKey,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }
    for m in p.api_gho.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::ApiKey,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }
    for m in p.api_github_pat.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::ApiKey,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }
    for m in p.api_bearer.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::ApiKey,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }

    // 6. IPv4 (with octet validation, exclude version-like strings)
    for caps in p.ipv4.captures_iter(input) {
        let m = caps.get(0).unwrap();
        // Skip if inside a version-like string
        let in_version = version_spans
            .iter()
            .any(|&(vs, ve)| m.start() >= vs && m.end() <= ve);
        if in_version {
            continue;
        }
        if valid_ipv4(
            caps.get(1).unwrap().as_str(),
            caps.get(2).unwrap().as_str(),
            caps.get(3).unwrap().as_str(),
            caps.get(4).unwrap().as_str(),
        ) {
            candidates.push(Candidate {
                kind: PiiKind::IpAddress,
                start: m.start(),
                end: m.end(),
                value: m.as_str().to_string(),
                skip: false,
            });
        }
    }

    // 7. IPv6
    for m in p.ipv6.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::IpAddress,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }

    // 8. Passport
    for m in p.passport_cn.find_iter(input) {
        candidates.push(Candidate {
            kind: PiiKind::Passport,
            start: m.start(),
            end: m.end(),
            value: m.as_str().to_string(),
            skip: false,
        });
    }
    for m in p.passport_general.find_iter(input) {
        // Skip if already matched as Chinese passport
        let overlaps = candidates.iter().any(|c| {
            !c.skip && c.kind == PiiKind::Passport && c.start <= m.start() && c.end >= m.end()
        });
        if !overlaps {
            candidates.push(Candidate {
                kind: PiiKind::Passport,
                start: m.start(),
                end: m.end(),
                value: m.as_str().to_string(),
                skip: false,
            });
        }
    }

    // 9. Bank card (16-19 digit Luhn-validated)
    for m in p.bank_card.find_iter(input) {
        let digits = m.as_str();
        // Skip if already matched as credit card or Chinese ID
        let overlaps = candidates.iter().any(|c| {
            !c.skip
                && (c.kind == PiiKind::CreditCard || c.kind == PiiKind::ChineseId)
                && c.start <= m.start()
                && c.end >= m.end()
        });
        if overlaps {
            continue;
        }
        if luhn_check(digits) {
            candidates.push(Candidate {
                kind: PiiKind::BankCard,
                start: m.start(),
                end: m.end(),
                value: digits.to_string(),
                skip: false,
            });
        }
    }

    // 10. Custom rules (Laputa stub)
    {
        let rules = custom_rules().read();
        for rule in rules.iter() {
            for m in rule.pattern.find_iter(input) {
                candidates.push(Candidate {
                    kind: PiiKind::Custom(rule.name.clone()),
                    start: m.start(),
                    end: m.end(),
                    value: m.as_str().to_string(),
                    skip: false,
                });
            }
        }
    }

    // Filter out skipped candidates, matches inside protected ids, then
    // resolve overlaps (keep longest match).
    candidates.retain(|c| !c.skip && !overlaps_span(c.start, c.end, &protected_id_spans));
    candidates.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut merged: Vec<Candidate> = Vec::new();
    for c in candidates {
        if let Some(last) = merged.last() {
            // Skip if this candidate overlaps with the previous (which starts earlier and is longer)
            if c.start < last.end {
                continue;
            }
        }
        merged.push(c);
    }

    // Resolve severity and build result
    let mut detected: Vec<PiiMatch> = Vec::new();
    let mut max_severity: Option<PiiSeverity> = None;

    for c in &merged {
        let sev = config.severity_for(&c.kind);
        detected.push(PiiMatch {
            kind: c.kind.clone(),
            value: c.value.clone(),
            start: c.start,
            end: c.end,
            severity: sev,
        });
        match (&max_severity, &sev) {
            (None, _) | (Some(PiiSeverity::Warning), PiiSeverity::Error) => {
                max_severity = Some(sev);
            }
            _ => {}
        }
    }

    // Check for error severity — if any, return early without redacting
    if max_severity == Some(PiiSeverity::Error) {
        return RedactionResult {
            redacted: input.to_string(),
            detected,
            max_severity,
        };
    }

    // Apply redaction for warning-severity items (replace right-to-left)
    let mut redacted = input.to_string();
    for m in detected.iter().rev() {
        let replacement = format!("[REDACTED:{}]", m.kind.label());
        redacted.replace_range(m.start..m.end, &replacement);
    }

    // Emit audit events per category
    let mut category_counts: std::collections::HashMap<String, (PiiSeverity, u32)> =
        std::collections::HashMap::new();
    for m in &detected {
        let label = m.kind.label().to_string();
        let entry = category_counts.entry(label).or_insert((m.severity, 0));
        entry.1 += 1;
    }
    for (category, (severity, count)) in category_counts {
        audit::emit(AuditEvent::PiiRedacted {
            category,
            severity,
            count,
        });
    }

    RedactionResult {
        redacted,
        detected,
        max_severity,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn warning_config() -> PiiConfig {
        PiiConfig {
            enabled: true,
            ..PiiConfig::default()
        }
    }

    fn error_config() -> PiiConfig {
        PiiConfig {
            default_severity: PiiSeverity::Error,
            enabled: true,
            category_severity: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_email_detection() {
        let config = warning_config();
        let result = redact_pii("Contact me at user@example.com please", &config);
        assert!(result.redacted.contains("[REDACTED:Email]"));
        assert!(!result.redacted.contains("user@example.com"));
        assert_eq!(result.detected.len(), 1);
        assert_eq!(result.detected[0].kind, PiiKind::Email);
    }

    #[test]
    fn test_chinese_phone_detection() {
        let config = warning_config();
        let result = redact_pii("My number is 13812345678", &config);
        assert!(result.redacted.contains("[REDACTED:Phone]"));
        assert!(!result.redacted.contains("13812345678"));
    }

    #[test]
    fn test_chinese_id_valid_checksum() {
        let config = warning_config();
        // Valid Chinese ID: 110101199003077037 (checksum validated)
        let result = redact_pii("ID: 110101199003077037", &config);
        assert!(result.redacted.contains("[REDACTED:ChineseId]"));
    }

    #[test]
    fn test_chinese_id_invalid_checksum() {
        let config = warning_config();
        // Invalid Chinese ID (wrong last digit)
        let result = redact_pii("ID: 110101199003077034", &config);
        assert!(!result.redacted.contains("[REDACTED:ChineseId]"));
    }

    #[test]
    fn test_credit_card_luhn() {
        let config = warning_config();
        // 4111111111111111 is a well-known Luhn-valid test card
        let result = redact_pii("Card: 4111111111111111", &config);
        assert!(result.redacted.contains("[REDACTED:CreditCard]"));

        // Invalid card number
        let result2 = redact_pii("Card: 4111111111111112", &config);
        assert!(!result2.redacted.contains("[REDACTED:CreditCard]"));
    }

    #[test]
    fn test_api_key_patterns() {
        let config = warning_config();

        let result = redact_pii("key=sk-abcdefghijklmnopqrstuvwx", &config);
        assert!(result.redacted.contains("[REDACTED:ApiKey]"));

        let result = redact_pii("key=pk-abcdefghijklmnopqrstuvwx", &config);
        assert!(result.redacted.contains("[REDACTED:ApiKey]"));

        let result = redact_pii(
            "token=ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            &config,
        );
        assert!(result.redacted.contains("[REDACTED:ApiKey]"));
    }

    #[test]
    fn test_ipv4_detection() {
        let config = warning_config();
        let result = redact_pii("Server at 192.168.1.1", &config);
        assert!(result.redacted.contains("[REDACTED:IpAddress]"));
    }

    #[test]
    fn test_ipv4_no_false_positive_on_version() {
        let config = warning_config();
        let result = redact_pii("version 1.2.3.4", &config);
        assert!(!result.redacted.contains("[REDACTED:IpAddress]"));
    }

    #[test]
    fn test_ipv6_detection() {
        let config = warning_config();
        let result = redact_pii("Server at 2001:0db8:85a3:0000:0000:8a2e:0370:7334", &config);
        assert!(result.redacted.contains("[REDACTED:IpAddress]"));
    }

    #[test]
    fn test_warning_severity_continues() {
        let config = warning_config();
        let result = redact_pii("Email: test@example.com and phone 13812345678", &config);
        assert_eq!(result.max_severity, Some(PiiSeverity::Warning));
        assert!(result.redacted.contains("[REDACTED:Email]"));
        assert!(result.redacted.contains("[REDACTED:Phone]"));
    }

    #[test]
    fn test_error_severity_rejects() {
        let config = error_config();
        let result = redact_pii("Email: test@example.com", &config);
        assert_eq!(result.max_severity, Some(PiiSeverity::Error));
        // Error severity does NOT redact — returns original input
        assert!(result.redacted.contains("test@example.com"));
        assert_eq!(result.detected.len(), 1);
    }

    #[test]
    fn test_redaction_result_structure() {
        let config = warning_config();
        let result = redact_pii("user@example.com", &config);
        assert!(!result.detected.is_empty());
        assert_eq!(result.max_severity, Some(PiiSeverity::Warning));
        let m = &result.detected[0];
        assert_eq!(m.kind, PiiKind::Email);
        assert!(m.start < m.end);
        assert!(!m.value.is_empty());
    }

    #[test]
    fn test_no_pii_detected() {
        let config = warning_config();
        let result = redact_pii("Hello, this is a safe message.", &config);
        assert_eq!(result.detected.len(), 0);
        assert_eq!(result.max_severity, None);
        assert_eq!(result.redacted, "Hello, this is a safe message.");
    }

    #[test]
    fn test_pii_disabled() {
        let config = PiiConfig {
            enabled: false,
            ..Default::default()
        };
        let result = redact_pii("user@example.com 13812345678", &config);
        assert_eq!(result.detected.len(), 0);
        assert_eq!(result.max_severity, None);
    }

    #[test]
    fn test_default_config_does_not_hide_content() {
        let result = redact_pii(
            "user@example.com 13812345678 memory-1786924800000000-a1b2c3d4e5f6",
            &PiiConfig::default(),
        );
        assert!(result.detected.is_empty());
        assert_eq!(
            result.redacted,
            "user@example.com 13812345678 memory-1786924800000000-a1b2c3d4e5f6"
        );
    }

    #[test]
    fn test_passport_detection() {
        let config = warning_config();
        let result = redact_pii("Passport E12345678", &config);
        assert!(result.redacted.contains("[REDACTED:Passport]"));
    }

    #[test]
    fn test_luhn_check() {
        assert!(luhn_check("4111111111111111"));
        assert!(luhn_check("4012888888881881"));
        assert!(!luhn_check("4111111111111112"));
        assert!(!luhn_check("0000000000000000")); // all zeros fails Luhn
    }

    #[test]
    fn test_chinese_id_checksum() {
        assert!(chinese_id_checksum("110101199003077037"));
        assert!(!chinese_id_checksum("110101199003077034"));
        assert!(!chinese_id_checksum("12345")); // too short
    }

    #[test]
    fn test_performance_large_input() {
        let config = warning_config();
        let mut input = String::with_capacity(100_000);
        for i in 0..2500 {
            input.push_str(&format!("Line {} with some text here. ", i));
        }
        input.push_str("secret: user@example.com");

        let start = std::time::Instant::now();
        let result = redact_pii(&input, &config);
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 200,
            "PII redaction took {:?}",
            elapsed
        );
        assert!(result.redacted.contains("[REDACTED:Email]"));
    }

    #[test]
    fn test_add_custom_rule() {
        let config = warning_config();
        // Clear custom rules for this test
        custom_rules().write().clear();

        add_custom_rule("test_rule", r"TEST-\d{4}", PiiSeverity::Warning).unwrap();
        let result = redact_pii("Found TEST-1234 in text", &config);
        assert!(result.redacted.contains("[REDACTED:test_rule]"));

        // Clean up
        custom_rules().write().clear();
    }

    #[test]
    fn test_add_custom_rule_invalid_pattern() {
        let result = add_custom_rule("bad", r"[invalid", PiiSeverity::Warning);
        assert!(result.is_err());
    }

    #[test]
    fn test_category_severity_override() {
        let mut config = warning_config();
        config
            .category_severity
            .insert("Email".to_string(), PiiSeverity::Error);
        // Phone stays at default (Warning)
        let result = redact_pii("user@example.com 13812345678", &config);
        // Error severity present → message rejected (not redacted)
        assert_eq!(result.max_severity, Some(PiiSeverity::Error));
        assert!(result.redacted.contains("user@example.com"));
    }

    #[test]
    fn test_bearer_token_detection() {
        let config = warning_config();
        let result = redact_pii(
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkw",
            &config,
        );
        assert!(result.redacted.contains("[REDACTED:ApiKey]"));
    }

    #[test]
    fn test_memory_record_id_timestamp_not_redacted_as_phone() {
        let config = warning_config();
        // 2026-era micros start with 17…, which used to match phone_cn (1[3-9] + 9 digits).
        let id = "memory-1786924800000000-a1b2c3d4e5f6";
        let result = redact_pii(&format!(r#"{{"id":"{id}","content":"note"}}"#), &config);
        assert!(
            result.redacted.contains(id),
            "memory record id must survive PII: {}",
            result.redacted
        );
        assert!(!result.redacted.contains("[REDACTED:Phone]"));
    }

    #[test]
    fn test_memory_record_id_luhn_timestamp_not_redacted_as_card() {
        let config = warning_config();
        // 16-digit Luhn-valid run in the current id shape must stay usable.
        let id = "memory-4111111111111111-abcdef123456";
        let result = redact_pii(id, &config);
        assert_eq!(result.redacted, id);
        assert!(result.detected.is_empty());
    }

    #[test]
    fn test_memory_import_and_tombstone_ids_are_preserved() {
        let config = warning_config();
        let input = concat!(
            "memory-laputa-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef ",
            "memory-legacy-fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210 ",
            "memory-tombstone-1786924800000000 ",
            "session-checkpoint-0123456789abcdef"
        );
        let result = redact_pii(input, &config);
        assert_eq!(result.redacted, input);
        assert!(result.detected.is_empty());
    }

    #[test]
    fn test_standalone_phone_still_redacted_next_to_memory_id() {
        let config = warning_config();
        let id = "memory-1786924800000000-a1b2c3d4e5f6";
        let result = redact_pii(&format!("{id} call 13812345678"), &config);
        assert!(result.redacted.contains(id));
        assert!(result.redacted.contains("[REDACTED:Phone]"));
        assert!(!result.redacted.contains("13812345678"));
    }

    #[test]
    fn test_phone_not_matched_inside_longer_digit_run() {
        let config = warning_config();
        // 16-digit run that is not a memory id and fails Luhn / card checks.
        let result = redact_pii("ref 1786924800000002 leftover", &config);
        assert!(
            !result.redacted.contains("[REDACTED:Phone]"),
            "phones must be isolated numbers, not substrings: {}",
            result.redacted
        );
        assert!(result.redacted.contains("1786924800000002"));
    }
}
