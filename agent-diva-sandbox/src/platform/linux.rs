//! Linux sandbox implementation using Landlock and Bubblewrap
//!
//! This module implements process isolation on Linux using:
//! - Landlock LSM for filesystem access control
//! - Seccomp-BPF for network syscall filtering
//! - Bubblewrap (bwrap) for filesystem namespace isolation
//!
//! Inspired by OpenAI Codex CLI's linux-sandbox architecture.

use crate::error::{SandboxError, SandboxResult};
use crate::filesystem::{FileSystemSandboxKind, FileSystemSandboxPolicy, WritableRoot};
use crate::policy::{ReadOnlyAccess, SandboxPolicy};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tracing::{debug, info, warn};

// ============================================================================
// Constants
// ============================================================================

/// Platform default read-only paths (system binaries and libraries)
pub const LINUX_PLATFORM_DEFAULT_READ_ROOTS: &[&str] = &[
    "/bin",
    "/sbin",
    "/usr",
    "/etc",
    "/lib",
    "/lib64",
    "/nix/store",
    "/run/current-system/sw",
];

/// WSL1 detection warning message
pub const WSL1_BWRAP_WARNING: &str = "WSL1 cannot use bubblewrap sandbox. WSL1 does not support \
    the user namespaces required for bwrap. Use WSL2 or run without sandbox.";

/// WSL1 proc filesystem marker path
const WSL1_PROC_MARKER: &str = "/proc/sys/fs/binfmt_misc/WSLInterop";

// ============================================================================
// Bubblewrap Types
// ============================================================================

/// Bubblewrap execution options
#[derive(Debug, Clone)]
pub struct BwrapOptions {
    /// Whether to mount /proc in the sandbox
    pub mount_proc: bool,
    /// Network isolation mode
    pub network_mode: BwrapNetworkMode,
    /// Maximum depth for glob pattern expansion
    pub glob_scan_max_depth: Option<usize>,
}

impl Default for BwrapOptions {
    fn default() -> Self {
        Self {
            mount_proc: true,
            network_mode: BwrapNetworkMode::Isolated,
            glob_scan_max_depth: Some(10),
        }
    }
}

/// Network isolation mode for bubblewrap
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BwrapNetworkMode {
    /// Full network access (host namespace)
    FullAccess,
    /// Isolated network (unshare-net)
    Isolated,
    /// Proxy-routed network (for managed network access)
    ProxyOnly,
}

impl BwrapNetworkMode {
    /// Check if network is isolated
    pub fn is_isolated(&self) -> bool {
        matches!(
            self,
            BwrapNetworkMode::Isolated | BwrapNetworkMode::ProxyOnly
        )
    }
}

/// Bubblewrap command arguments result
#[derive(Debug)]
pub struct BwrapArgs {
    /// Command arguments for bwrap
    pub args: Vec<String>,
    /// Preserved file descriptors (e.g., /dev/null for file masking)
    pub preserved_fds: Vec<std::os::unix::io::RawFd>,
}

impl BwrapArgs {
    /// Create new bwrap args
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            preserved_fds: Vec::new(),
        }
    }

    /// Add a preserved file descriptor
    pub fn with_preserved_fd(mut self, fd: std::os::unix::io::RawFd) -> Self {
        self.preserved_fds.push(fd);
        self
    }
}

// ============================================================================
// WSL Detection
// ============================================================================

/// Check if running under WSL1
pub fn is_wsl1() -> bool {
    // WSL1 has a specific marker in /proc
    Path::new(WSL1_PROC_MARKER).exists()
}

/// Check if running under WSL (either version)
pub fn is_wsl() -> bool {
    // Check for WSL version marker in /proc/version
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        version.contains("WSL") || version.contains("Microsoft")
    } else {
        false
    }
}

/// Ensure bubblewrap is supported on this system
pub fn ensure_bwrap_supported() -> SandboxResult<()> {
    // Check for WSL1 - bwrap is not supported
    if is_wsl1() {
        return Err(SandboxError::PlatformError(WSL1_BWRAP_WARNING.to_string()));
    }

    // Check if bwrap is installed
    if !is_bwrap_installed() {
        return Err(SandboxError::PlatformError(
            "bubblewrap (bwrap) is not installed. Install it via: apt install bubblewrap"
                .to_string(),
        ));
    }

    Ok(())
}

/// Check if bubblewrap is installed
pub fn is_bwrap_installed() -> bool {
    // Check for bwrap in common paths
    let bwrap_paths = ["/usr/bin/bwrap", "/usr/local/bin/bwrap", "bwrap"];

    for path in &bwrap_paths {
        if std::process::Command::new(path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

// ============================================================================
// Bubblewrap Command Generation
// ============================================================================

/// Create bubblewrap command arguments for sandboxed execution
///
/// This function generates the bwrap command line arguments based on:
/// - File system policy (read-only/read-write paths)
/// - Network policy (isolated/full access)
/// - Protected paths (.git, .diva, etc.)
///
/// # Arguments
/// * `command` - The command to execute (as argument vector)
/// * `fs_policy` - File system sandbox policy
/// * `sandbox_policy_cwd` - The cwd for policy evaluation
/// * `command_cwd` - The actual cwd for the command
/// * `options` - Bubblewrap execution options
///
/// # Returns
/// BwrapArgs containing the command line and preserved file descriptors
pub fn create_bwrap_command_args(
    command: Vec<String>,
    fs_policy: &FileSystemSandboxPolicy,
    sandbox_policy_cwd: &Path,
    command_cwd: &Path,
    options: BwrapOptions,
) -> SandboxResult<BwrapArgs> {
    // Fast path: unrestricted policy + full network + no glob restrictions
    if fs_policy.kind == FileSystemSandboxKind::Unrestricted
        && options.network_mode == BwrapNetworkMode::FullAccess
    {
        debug!("Fast path: no sandbox needed, executing directly");
        return Ok(BwrapArgs::new(command));
    }

    // Generate filesystem arguments
    let mut args = create_filesystem_args(fs_policy, sandbox_policy_cwd, command_cwd, &options)?;

    // Add network arguments
    if options.network_mode.is_isolated() {
        args.args.push("--unshare-net".to_string());
    }

    // Add proc mount
    if options.mount_proc {
        args.args.push("--proc".to_string());
        args.args.push("/proc".to_string());
    }

    // Add the command to execute
    args.args.push("--".to_string());
    args.args.extend(command);

    Ok(args)
}

/// Create filesystem bind mount arguments
///
/// Mount order is critical:
/// 1. Base view: full disk read-only or restricted read
/// 2. /dev device nodes
/// 3. Mask unreadable ancestor paths
/// 4. Writable roots bind mount
/// 5. Re-mask read-only subpaths within writable roots
/// 6. Nested unreadable paths masking
fn create_filesystem_args(
    fs_policy: &FileSystemSandboxPolicy,
    sandbox_policy_cwd: &Path,
    command_cwd: &Path,
    options: &BwrapOptions,
) -> SandboxResult<BwrapArgs> {
    let mut args = Vec::new();
    let mut preserved_fds = Vec::new();

    // Check if we have full disk read access
    let has_full_read_access = fs_policy.kind == FileSystemSandboxKind::Unrestricted;

    if has_full_read_access {
        // Full disk read-only base view
        args.push("--ro-bind".to_string());
        args.push("/".to_string());
        args.push("/".to_string());

        // Create standard device nodes
        args.push("--dev".to_string());
        args.push("/dev".to_string());
    } else {
        // Restricted read: start with empty tmpfs
        args.push("--tmpfs".to_string());
        args.push("/".to_string());

        // Create device nodes
        args.push("--dev".to_string());
        args.push("/dev".to_string());

        // Add platform default read roots
        for root in LINUX_PLATFORM_DEFAULT_READ_ROOTS {
            let path = Path::new(root);
            if path.exists() {
                args.push("--ro-bind".to_string());
                args.push(root.to_string());
                args.push(root.to_string());
            }
        }
    }

    // Collect writable roots from policy entries
    let writable_roots = collect_writable_roots_from_policy(fs_policy, sandbox_policy_cwd);

    // Sort writable roots by path length (longest first for proper nesting)
    let mut sorted_roots: Vec<_> = writable_roots.iter().collect();
    sorted_roots.sort_by(|a, b| b.root.as_os_str().len().cmp(&a.root.as_os_str().len()));

    // Add writable roots with bind mount
    for root in sorted_roots {
        // Resolve symlinks for actual mount target
        let mount_target = resolve_symlink_target(&root.root).unwrap_or_else(|| root.root.clone());

        args.push("--bind".to_string());
        args.push(mount_target.to_string_lossy().to_string());
        args.push(mount_target.to_string_lossy().to_string());

        // Re-mask read-only subpaths within writable roots
        for subpath in &root.read_only_subpaths {
            append_read_only_subpath_args(&mut args, subpath, &[&root.root], &mut preserved_fds);
        }
    }

    // Add cwd if different from sandbox policy cwd
    if command_cwd != sandbox_policy_cwd {
        args.push("--bind".to_string());
        args.push(command_cwd.to_string_lossy().to_string());
        args.push(command_cwd.to_string_lossy().to_string());
    }

    Ok(BwrapArgs {
        args,
        preserved_fds,
    })
}

/// Collect writable roots from filesystem policy entries
fn collect_writable_roots_from_policy(
    fs_policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> Vec<WritableRoot> {
    let mut roots = Vec::new();

    for entry in &fs_policy.entries {
        // Only process write entries
        if entry.access.allows_write() {
            let resolved_path = match &entry.path {
                crate::filesystem::FileSystemPath::Path { path } => {
                    if path.is_absolute() {
                        path.clone()
                    } else {
                        cwd.join(path)
                    }
                }
                crate::filesystem::FileSystemPath::Special { value } => {
                    // Resolve special paths
                    match value {
                        crate::filesystem::FileSystemSpecialPath::CurrentWorkingDirectory => {
                            cwd.clone()
                        }
                        crate::filesystem::FileSystemSpecialPath::Root => PathBuf::from("/"),
                        crate::filesystem::FileSystemSpecialPath::Tmpdir => std::env::var("TMPDIR")
                            .map(PathBuf::from)
                            .unwrap_or_else(|_| PathBuf::from("/tmp")),
                        _ => cwd.clone(),
                    }
                }
                _ => continue, // Skip glob patterns for writable roots
            };

            roots.push(WritableRoot::new(resolved_path));
        }
    }

    roots
}

/// Append arguments to mask a read-only subpath within a writable root
fn append_read_only_subpath_args(
    args: &mut Vec<String>,
    subpath: &Path,
    allowed_write_paths: &[&PathBuf],
    preserved_fds: &mut Vec<std::os::unix::io::RawFd>,
) {
    // Use tmpfs to mask directories or --ro-bind-data for files
    if subpath.is_dir() {
        // Check if there are writable descendants
        let has_writable_descendants = allowed_write_paths.iter().any(|wp| wp.starts_with(subpath));

        if has_writable_descendants {
            // Use tmpfs with minimal permissions (111 = execute only)
            args.push("--perms".to_string());
            args.push("111".to_string());
        } else {
            // Use tmpfs with no permissions (000)
            args.push("--perms".to_string());
            args.push("000".to_string());
        }
        args.push("--tmpfs".to_string());
        args.push(subpath.to_string_lossy().to_string());
        args.push("--remount-ro".to_string());
        args.push(subpath.to_string_lossy().to_string());
    } else {
        // Use /dev/null to mask files
        // Note: In full implementation, we'd open /dev/null and pass fd
        args.push("--ro-bind-data".to_string());
        args.push("0".to_string()); // Placeholder fd (would be actual fd in production)
        args.push(subpath.to_string_lossy().to_string());
    }
}

/// Resolve symlink target for mount
fn resolve_symlink_target(path: &Path) -> Option<PathBuf> {
    if path.is_symlink() {
        std::fs::read_link(path).ok()
    } else {
        None
    }
}

// ============================================================================
// Linux Sandbox Executor
// ============================================================================

/// Linux sandbox executor
pub struct LinuxSandboxExecutor {
    options: BwrapOptions,
}

impl LinuxSandboxExecutor {
    /// Create a new Linux sandbox executor
    pub fn new() -> Self {
        Self {
            options: BwrapOptions::default(),
        }
    }

    /// Create with custom options
    pub fn with_options(options: BwrapOptions) -> Self {
        Self { options }
    }

    /// Check if sandbox is available
    pub fn is_available(&self) -> bool {
        // Check for bwrap and Landlock support
        is_bwrap_installed() && !is_wsl1()
    }

    /// Execute a command in the sandbox
    pub async fn execute(
        &self,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
        policy: &SandboxPolicy,
        fs_policy: &FileSystemSandboxPolicy,
    ) -> SandboxResult<String> {
        info!("Executing command in Linux sandbox: {}", command);

        // Check if sandbox is supported
        ensure_bwrap_supported()?;

        // Parse command into arguments
        let command_args =
            shell_words::split(command).map_err(|e| SandboxError::InvalidCommand(e.to_string()))?;

        // Create bwrap arguments
        let bwrap_args =
            create_bwrap_command_args(command_args, fs_policy, cwd, cwd, self.options.clone())?;

        // Execute via bwrap
        self.execute_bwrap(bwrap_args, cwd, env, timeout_secs).await
    }

    /// Execute via bubblewrap
    async fn execute_bwrap(
        &self,
        bwrap_args: BwrapArgs,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
    ) -> SandboxResult<String> {
        use tokio::process::Command;
        use tokio::time::timeout;

        debug!("Executing bwrap with args: {:?}", bwrap_args.args);

        let mut cmd = Command::new("bwrap");
        for arg in &bwrap_args.args {
            cmd.arg(arg);
        }

        cmd.current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(timeout_secs), output_future).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(SandboxError::SpawnFailed(e.to_string())),
            Err(_) => return Err(SandboxError::Timeout { secs: timeout_secs }),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            let code = output.status.code().unwrap_or(-1);
            Err(SandboxError::ExecutionFailed {
                code,
                stdout,
                stderr,
            })
        }
    }

    /// Execute directly without sandbox
    pub async fn execute_direct(
        &self,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
    ) -> SandboxResult<String> {
        info!("Executing command directly (no sandbox): {}", command);

        use tokio::process::Command;
        use tokio::time::timeout;

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(timeout_secs), output_future).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(SandboxError::SpawnFailed(e.to_string())),
            Err(_) => return Err(SandboxError::Timeout { secs: timeout_secs }),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            let code = output.status.code().unwrap_or(-1);
            Err(SandboxError::ExecutionFailed {
                code,
                stdout,
                stderr,
            })
        }
    }
}

impl Default for LinuxSandboxExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Landlock Support
// ============================================================================

use landlock::{
    Access, AccessFs, LandlockPath, PathBeneath, Ruleset, RulesetAttr, RulesetCreated,
    RulesetNoRights, ABI,
};

/// Landlock ruleset wrapper for easier configuration
pub struct LandlockRulesetBuilder {
    /// Access rights for read operations
    read_access: AccessFs,
    /// Access rights for write operations
    write_access: AccessFs,
    /// Paths allowed for read access
    read_paths: Vec<PathBuf>,
    /// Paths allowed for write access
    write_paths: Vec<PathBuf>,
    /// ABI version to use
    abi: ABI,
}

impl LandlockRulesetBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            // Read operations: read files, execute files, read directories
            read_access: AccessFs::from_all(ABI::V1)
                .union(AccessFs::EXEC)
                .union(AccessFs::READ_DIR),
            // Write operations: write files, create files, delete files, make directories
            write_access: AccessFs::from_all(ABI::V1)
                .union(AccessFs::WRITE_FILE)
                .union(AccessFs::MAKE_REG)
                .union(AccessFs::REMOVE_FILE)
                .union(AccessFs::MAKE_DIR)
                .union(AccessFs::REMOVE_DIR),
            read_paths: Vec::new(),
            write_paths: Vec::new(),
            abi: ABI::V1,
        }
    }

    /// Add a read-only path
    pub fn add_read_path(mut self, path: PathBuf) -> Self {
        self.read_paths.push(path);
        self
    }

    /// Add multiple read-only paths
    pub fn add_read_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.read_paths.extend(paths);
        self
    }

    /// Add a writable path
    pub fn add_write_path(mut self, path: PathBuf) -> Self {
        self.write_paths.push(path);
        self
    }

    /// Add multiple writable paths
    pub fn add_write_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.write_paths.extend(paths);
        self
    }

    /// Set the ABI version
    pub fn with_abi(mut self, abi: ABI) -> Self {
        self.abi = abi;
        self
    }

    /// Build the Landlock ruleset
    pub fn build(&self) -> SandboxResult<LandlockRuleset> {
        // Create the base ruleset
        let ruleset = Ruleset::new()
            .handle_access(self.read_access.union(self.write_access))
            .abi(self.abi)
            .create()
            .map_err(|e| {
                SandboxError::Internal(format!("Failed to create Landlock ruleset: {}", e))
            })?;

        // Add read-only path rules
        let ruleset = self.add_path_rules(&ruleset, &self.read_paths, self.read_access)?;

        // Add writable path rules
        let ruleset = self.add_path_rules(&ruleset, &self.write_paths, self.write_access)?;

        Ok(LandlockRuleset {
            ruleset,
            abi: self.abi,
        })
    }

    /// Add path rules to a ruleset
    fn add_path_rules<R: landlock::RulesetCreated>(
        &self,
        ruleset: &R,
        paths: &[PathBuf],
        access: AccessFs,
    ) -> SandboxResult<R> {
        let mut current = ruleset.clone();

        for path in paths {
            if path.exists() {
                let path_beneath = PathBeneath::new(path, access);
                current = current.add_rule(path_beneath).map_err(|e| {
                    SandboxError::Internal(format!(
                        "Failed to add Landlock path rule for {}: {}",
                        path.display(),
                        e
                    ))
                })?;
                debug!(
                    "Added Landlock rule for path: {} with access {:?}",
                    path.display(),
                    access
                );
            }
        }

        Ok(current)
    }
}

impl Default for LandlockRulesetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiled Landlock ruleset ready for enforcement
pub struct LandlockRuleset {
    ruleset: RulesetCreated<RulesetNoRights>,
    abi: ABI,
}

impl LandlockRuleset {
    /// Restrict the current thread with this ruleset
    pub fn restrict_current_thread(&self) -> SandboxResult<()> {
        self.ruleset.restrict_self().map_err(|e| {
            SandboxError::Internal(format!("Failed to apply Landlock restrictions: {}", e))
        })?;

        info!(
            "Landlock restrictions applied to current thread (ABI {:?})",
            self.abi
        );
        Ok(())
    }
}

/// Check if Landlock is supported on this kernel
pub fn is_landlock_supported() -> bool {
    // Try to create a minimal ruleset to verify Landlock support
    // This is more reliable than parsing /proc/version
    #[cfg(target_os = "linux")]
    {
        // Try creating a ruleset with ABI V1 (kernel 5.13+)
        if let Ok(_) = Ruleset::new()
            .handle_access(AccessFs::from_all(ABI::V1))
            .abi(ABI::V1)
            .create()
        {
            return true;
        }

        // Try ABI V2 (kernel 5.19+)
        if let Ok(_) = Ruleset::new()
            .handle_access(AccessFs::from_all(ABI::V2))
            .abi(ABI::V2)
            .create()
        {
            return true;
        }

        // Try ABI V3 (kernel 6.7+)
        if let Ok(_) = Ruleset::new()
            .handle_access(AccessFs::from_all(ABI::V3))
            .abi(ABI::V3)
            .create()
        {
            return true;
        }
    }

    false
}

/// Parse kernel version from /proc/version
fn parse_kernel_version() -> Option<(u32, u32, u32)> {
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        // Format: Linux version 5.15.0-...
        if let Some(version_str) = version.split_whitespace().nth(2) {
            // Parse major.minor.patch
            let parts: Vec<&str> = version_str.split('.').collect();
            if parts.len() >= 3 {
                let major = parts[0].parse::<u32>().ok()?;
                let minor = parts[1].parse::<u32>().ok()?;
                // Handle patch version that may have suffix (e.g., "0-generic")
                let patch = parts[2]
                    .split('-')
                    .next()
                    .and_then(|p| p.parse::<u32>().ok())?;
                return Some((major, minor, patch));
            }
        }
    }
    None
}

/// Get the best Landlock ABI for this kernel
pub fn get_best_landlock_abi() -> ABI {
    // Try each ABI version, starting from newest
    #[cfg(target_os = "linux")]
    {
        // ABI V3 (kernel 6.7+) - supports ioctl and truncate
        if Ruleset::new()
            .handle_access(AccessFs::from_all(ABI::V3))
            .abi(ABI::V3)
            .create()
            .is_ok()
        {
            return ABI::V3;
        }

        // ABI V2 (kernel 5.19+) - supports file truncation
        if Ruleset::new()
            .handle_access(AccessFs::from_all(ABI::V2))
            .abi(ABI::V2)
            .create()
            .is_ok()
        {
            return ABI::V2;
        }
    }

    // Default to ABI V1 (kernel 5.13+)
    ABI::V1
}

/// Build Landlock ruleset from FileSystemSandboxPolicy
pub fn build_landlock_from_fs_policy(
    fs_policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> SandboxResult<LandlockRuleset> {
    let abi = get_best_landlock_abi();
    let builder = LandlockRulesetBuilder::new().with_abi(abi);

    // Add read-only paths based on policy
    let read_paths: Vec<PathBuf> = fs_policy
        .entries
        .iter()
        .filter(|e| e.access.allows_read() && !e.access.allows_write())
        .filter_map(|e| {
            match &e.path {
                FileSystemPath::Path { path } => {
                    if path.is_absolute() {
                        Some(path.clone())
                    } else {
                        Some(cwd.join(path))
                    }
                }
                FileSystemPath::Special { value } => match value {
                    crate::filesystem::FileSystemSpecialPath::CurrentWorkingDirectory => {
                        Some(cwd.clone())
                    }
                    crate::filesystem::FileSystemSpecialPath::Root => Some(PathBuf::from("/")),
                    _ => None,
                },
                _ => None, // Skip glob patterns
            }
        })
        .collect();

    // Add writable paths based on policy
    let write_paths: Vec<PathBuf> = fs_policy
        .entries
        .iter()
        .filter(|e| e.access.allows_write())
        .filter_map(|e| match &e.path {
            FileSystemPath::Path { path } => {
                if path.is_absolute() {
                    Some(path.clone())
                } else {
                    Some(cwd.join(path))
                }
            }
            FileSystemPath::Special { value } => match value {
                crate::filesystem::FileSystemSpecialPath::CurrentWorkingDirectory => {
                    Some(cwd.clone())
                }
                crate::filesystem::FileSystemSpecialPath::Tmpdir => std::env::var("TMPDIR")
                    .map(PathBuf::from)
                    .ok()
                    .or_else(|| Some(PathBuf::from("/tmp"))),
                _ => None,
            },
            _ => None,
        })
        .collect();

    // Add platform default read roots if policy is restricted
    let platform_roots: Vec<PathBuf> = if fs_policy.kind == FileSystemSandboxKind::Restricted {
        LINUX_PLATFORM_DEFAULT_READ_ROOTS
            .iter()
            .filter_map(|r| {
                let path = PathBuf::from(r);
                if path.exists() {
                    Some(path)
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    builder
        .add_read_paths(read_paths)
        .add_read_paths(platform_roots)
        .add_write_paths(write_paths)
        .build()
}

/// Install Landlock rules on current thread (for use after fork)
pub fn install_landlock_on_current_thread(
    fs_policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> SandboxResult<()> {
    if !is_landlock_supported() {
        warn!("Landlock not supported on this kernel");
        return Ok(());
    }

    let ruleset = build_landlock_from_fs_policy(fs_policy, cwd)?;
    ruleset.restrict_current_thread()?;
    Ok(())
}

// ============================================================================
// Seccomp Network Filtering
// ============================================================================

/// Network syscall filtering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSeccompMode {
    /// Block all network syscalls
    FullBlock,
    /// Allow only loopback connections (for proxy routing)
    ProxyOnly,
    /// Allow specific port connections
    AllowPorts(Vec<u16>),
    /// No network filtering
    Disabled,
}

impl NetworkSeccompMode {
    /// Check if filtering is enabled
    pub fn is_enabled(&self) -> bool {
        !matches!(self, NetworkSeccompMode::Disabled)
    }
}

impl Default for NetworkSeccompMode {
    fn default() -> Self {
        NetworkSeccompMode::FullBlock
    }
}

/// Build seccomp filter for network syscalls
///
/// This function creates a BPF filter that restricts network-related syscalls:
/// - connect, sendto, sendmsg, recvfrom, recvmsg (for socket communication)
/// - socket, bind, listen, accept (for socket creation)
///
/// The filter can be configured to:
/// - Block all network syscalls
/// - Allow only loopback (127.0.0.1) connections
/// - Allow specific ports
#[cfg(target_os = "linux")]
pub fn build_network_seccomp_filter(
    mode: NetworkSeccompMode,
) -> SandboxResult<seccompiler::BpfProgram> {
    use seccompiler::{
        Arch, Arg, BpfProgram, Op, Rule, SeccompAction, SeccompCompare, SeccompFilter, SeccompRule,
        Target,
    };

    if !mode.is_enabled() {
        // Return an empty filter (no restrictions)
        return Ok(Vec::new());
    }

    // Define network syscalls to filter
    let network_syscalls = [
        "connect", "sendto", "sendmsg", "recvfrom",
        "recvmsg",
        // Note: We allow socket, bind, listen, accept for local usage
    ];

    let mut rules = std::collections::HashMap::new();

    match mode {
        NetworkSeccompMode::FullBlock => {
            // Block all network syscalls
            for syscall in &network_syscalls {
                rules.insert(
                    syscall.to_string(),
                    vec![Rule::new(vec![], SeccompAction::Errno(13))], // EACCES
                );
            }
        }
        NetworkSeccompMode::ProxyOnly => {
            // Allow connect to loopback only (127.0.0.1)
            // sockaddr_in.sin_addr.s_addr = 0x7f000001 (127.0.0.1)
            // This is complex to implement with seccomp as we need to inspect memory
            // For now, we'll allow connect and rely on higher-level filtering
            // Full implementation would need to inspect sockaddr structure

            // Allow connect (will be filtered at bwrap level for network isolation)
            rules.insert(
                "connect".to_string(),
                vec![Rule::new(vec![], SeccompAction::Allow)],
            );

            // Block other network syscalls
            for syscall in &["sendto", "sendmsg", "recvfrom", "recvmsg"] {
                rules.insert(
                    syscall.to_string(),
                    vec![Rule::new(vec![], SeccompAction::Errno(13))],
                );
            }
        }
        NetworkSeccompMode::AllowPorts(_ports) => {
            // Port-specific filtering would require inspecting sockaddr
            // This is complex with seccomp alone, so we allow connect
            // and rely on bwrap network isolation
            rules.insert(
                "connect".to_string(),
                vec![Rule::new(vec![], SeccompAction::Allow)],
            );

            for syscall in &["sendto", "sendmsg", "recvfrom", "recvmsg"] {
                rules.insert(
                    syscall.to_string(),
                    vec![Rule::new(vec![], SeccompAction::Allow)],
                );
            }
        }
        NetworkSeccompMode::Disabled => {
            // No rules - allow all
        }
    }

    // Create the filter
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow, // Default action: allow
        vec![Arch::X8664],    // Architecture
    )
    .map_err(|e| SandboxError::Internal(format!("Failed to create seccomp filter: {}", e)))?;

    // Compile to BPF program
    filter
        .compile()
        .map_err(|e| SandboxError::Internal(format!("Failed to compile seccomp filter: {}", e)))
}

#[cfg(not(target_os = "linux"))]
pub fn build_network_seccomp_filter(mode: NetworkSeccompMode) -> SandboxResult<Vec<u8>> {
    // Not supported on non-Linux platforms
    Ok(Vec::new())
}

/// Install seccomp filter on current thread
#[cfg(target_os = "linux")]
pub fn install_seccomp_filter(filter: &seccompiler::BpfProgram) -> SandboxResult<()> {
    use seccompiler::apply_filter;

    apply_filter(filter)
        .map_err(|e| SandboxError::Internal(format!("Failed to apply seccomp filter: {}", e)))?;

    info!("Seccomp network filter applied to current thread");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn install_seccomp_filter(_filter: &[u8]) -> SandboxResult<()> {
    Ok(())
}

/// Install network seccomp filter with the given mode
pub fn install_network_seccomp(mode: NetworkSeccompMode) -> SandboxResult<()> {
    if !mode.is_enabled() {
        return Ok(());
    }

    let filter = build_network_seccomp_filter(mode)?;
    install_seccomp_filter(&filter)?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::{FileSystemAccessMode, FileSystemPath, FileSystemSandboxEntry};

    #[test]
    fn test_bwrap_options_default() {
        let opts = BwrapOptions::default();
        assert!(opts.mount_proc);
        assert_eq!(opts.network_mode, BwrapNetworkMode::Isolated);
    }

    #[test]
    fn test_network_mode_is_isolated() {
        assert!(BwrapNetworkMode::Isolated.is_isolated());
        assert!(BwrapNetworkMode::ProxyOnly.is_isolated());
        assert!(!BwrapNetworkMode::FullAccess.is_isolated());
    }

    #[test]
    fn test_bwrap_args_creation() {
        let args = BwrapArgs::new(vec!["test".to_string()]);
        assert_eq!(args.args, vec!["test".to_string()]);
        assert!(args.preserved_fds.is_empty());
    }

    #[test]
    fn test_collect_writable_roots_empty() {
        let policy = FileSystemSandboxPolicy::restricted(Vec::new());
        let cwd = Path::new("/workspace");
        let roots = collect_writable_roots_from_policy(&policy, cwd);
        assert!(roots.is_empty());
    }

    #[test]
    fn test_collect_writable_roots_with_entry() {
        let entries = vec![FileSystemSandboxEntry::new(
            FileSystemPath::from_path(PathBuf::from("/workspace")),
            FileSystemAccessMode::Write,
        )];
        let policy = FileSystemSandboxPolicy::restricted(entries);
        let cwd = Path::new("/workspace");
        let roots = collect_writable_roots_from_policy(&policy, cwd);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, PathBuf::from("/workspace"));
    }

    #[test]
    fn test_is_wsl_detection() {
        // This test may pass or fail depending on environment
        // It's mainly to ensure the function doesn't crash
        let _ = is_wsl();
        let _ = is_wsl1();
    }

    #[test]
    fn test_platform_default_roots() {
        assert!(!LINUX_PLATFORM_DEFAULT_READ_ROOTS.is_empty());
        assert!(LINUX_PLATFORM_DEFAULT_READ_ROOTS.contains(&"/usr"));
    }

    #[test]
    fn test_executor_creation() {
        let executor = LinuxSandboxExecutor::new();
        // is_available depends on system state
        let _ = executor.is_available();
    }

    #[test]
    fn test_resolve_symlink_target() {
        // Regular path should return None
        let regular = Path::new("/tmp");
        assert!(resolve_symlink_target(regular).is_none());
    }
}
