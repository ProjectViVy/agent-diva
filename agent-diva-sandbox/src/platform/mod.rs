//! Platform-specific sandbox implementations

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

/// Sandbox type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxType {
    /// No sandbox (direct execution)
    None,
    /// Windows Restricted Token
    #[cfg(windows)]
    WindowsRestrictedToken,
    /// Linux Landlock/Seccomp
    #[cfg(target_os = "linux")]
    LinuxSeccomp,
    /// macOS Seatbelt
    #[cfg(target_os = "macos")]
    MacosSeatbelt,
}

// Manual Default impl needed due to cfg-dependent variants
#[allow(clippy::derivable_impls)]
impl Default for SandboxType {
    fn default() -> Self {
        #[cfg(windows)]
        {
            SandboxType::WindowsRestrictedToken
        }
        #[cfg(target_os = "linux")]
        {
            SandboxType::LinuxSeccomp
        }
        #[cfg(target_os = "macos")]
        {
            SandboxType::MacosSeatbelt
        }
        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            SandboxType::None
        }
    }
}

/// Get the current platform's sandbox type
pub fn current_platform_sandbox_type() -> SandboxType {
    SandboxType::default()
}
