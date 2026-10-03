use std::env;
use std::fs;

/// Keeps the Rust ABI constant honest against the staged `vivy_abi.h`.
/// Set `VIVY_ABI_HEADER=/path/to/vivy_abi.h` in builds that bundle the
/// artifact; the build fails if the header's VIVY_ABI_VERSION no longer
/// matches the frozen value compiled into the crate.
fn main() {
    println!("cargo:rerun-if-env-changed=VIVY_ABI_HEADER");
    let Ok(header) = env::var("VIVY_ABI_HEADER") else {
        println!("cargo:warning=VIVY_ABI_HEADER unset; using frozen ABI 1");
        return;
    };
    let text = fs::read_to_string(&header)
        .unwrap_or_else(|e| panic!("read VIVY_ABI_HEADER {header}: {e}"));
    let version = parse_abi_version(&text)
        .unwrap_or_else(|| panic!("VIVY_ABI_VERSION not defined in {header}"));
    if version != 1 {
        panic!("VIVY_ABI_VERSION {version} in {header}, want 1");
    }
}

fn parse_abi_version(text: &str) -> Option<u32> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#define VIVY_ABI_VERSION") {
            return rest.trim().parse().ok();
        }
    }
    None
}
