use super::patch_types::PatchSuccess;

pub(crate) fn format_success(path: &str, success: &PatchSuccess) -> String {
    format!(
        "Successfully patched {}\nStrategy: {}\nDiff:\n{}",
        path,
        success.strategy.as_str(),
        build_diff(path, &success.matched_text, &success.replacement_text)
    )
}

fn build_diff(path: &str, old_text: &str, new_text: &str) -> String {
    let mut diff = String::new();
    diff.push_str(&format!("--- {}\n", path));
    diff.push_str(&format!("+++ {}\n", path));
    diff.push_str("@@ patch @@\n");
    for line in old_text.lines() {
        diff.push('-');
        diff.push_str(line);
        diff.push('\n');
    }
    for line in new_text.lines() {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    diff.trim_end().to_string()
}
