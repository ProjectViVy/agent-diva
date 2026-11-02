use std::sync::{Arc, Mutex};
#[derive(Clone, Default)]
pub struct Redactor(Arc<Mutex<Vec<String>>>);
impl Redactor {
    pub fn stored_keys(&self, home: &std::path::Path) {
        if let Ok(entries) = std::fs::read_dir(home.join("auth/provider-keys")) {
            for entry in entries.flatten().take(256) {
                if entry.path().extension().is_some_and(|e| e == "json")
                    && entry
                        .metadata()
                        .is_ok_and(|m| m.is_file() && m.len() <= 65536)
                {
                    if let Ok(bytes) = std::fs::read(entry.path()) {
                        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if let Some(key) = value["key"].as_str() {
                                self.register(key);
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn environment() -> Self {
        let result = Self::default();
        for (name, value) in std::env::vars() {
            if ["KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL"]
                .iter()
                .any(|word| name.to_ascii_uppercase().contains(word))
            {
                result.register(&value);
            }
        }
        result
    }
    pub fn register(&self, value: &str) {
        if value.len() >= 4 {
            let mut secrets = self.0.lock().unwrap();
            if !secrets.iter().any(|s| s == value) {
                secrets.push(value.into());
                secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
            }
        }
    }
    pub fn text(&self, input: &str) -> String {
        let mut value = input.to_owned();
        for secret in self.0.lock().unwrap().iter() {
            value = value.replace(secret, "[redacted]");
        }
        let ansi = regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap();
        value = ansi.replace_all(&value, "").into_owned();
        let credential=regex::Regex::new(r"(?i)(bearer\s+|(?:api[_-]?key|token|password|secret)\s*[:=]\s*)[^\s,;\x22]+|\bsk-[a-zA-Z0-9_-]{8,}").unwrap();
        credential.replace_all(&value, "[redacted]").into_owned()
    }
}
pub fn append_tail(tail: &mut String, text: &str) {
    tail.push_str(text);
    if tail.len() > 8192 {
        let mut start = tail.len() - 8192;
        while !tail.is_char_boundary(start) {
            start += 1;
        }
        tail.drain(..start);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_causes_and_redacts_registered_and_common_credentials() {
        let r = Redactor::default();
        r.register("private-fixture-value");
        let text=r.text("SessionGuard: corrupt log at row 8; private-fixture-value; Authorization: Bearer unknown-token; sk-abcdefgh123");
        assert!(text.contains("corrupt log at row 8"));
        assert!(!text.contains("private-fixture-value"));
        assert!(!text.contains("unknown-token"));
        assert!(!text.contains("sk-abcdefgh123"));
    }
    #[test]
    fn diagnostics_are_bounded_on_unicode_boundaries() {
        let mut tail = String::new();
        append_tail(&mut tail, &"错误".repeat(5000));
        assert!(tail.len() <= 8192);
        assert!(tail.ends_with("错误"));
    }
}
