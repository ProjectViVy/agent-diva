//! Mentle toolkit discovery helpers for configuration UIs.

use std::path::Path;

/// Whether Mentle toolkit discovery is compiled into this build.
#[must_use]
pub const fn mentle_discovery_available() -> bool {
    cfg!(feature = "mentle")
}

/// Discover `memtle_*` tool names from the workspace toolkit metadata.
pub async fn discover_mentle_tool_names(workspace: &Path) -> Vec<String> {
    #[cfg(feature = "mentle")]
    {
        return crate::mentle_runtime::discover_mentle_tool_names(workspace).await;
    }

    #[cfg(not(feature = "mentle"))]
    {
        let _ = workspace;
        Vec::new()
    }
}
