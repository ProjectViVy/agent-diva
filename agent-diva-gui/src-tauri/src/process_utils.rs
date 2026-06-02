//! Legacy external gateway cleanup helpers.
//!
//! The embedded gateway is now the primary runtime path for the GUI. This
//! module remains only for debug-mode compatibility and best-effort cleanup of
//! stray legacy gateway processes during destructive maintenance flows such as
//! local data wipe.
#![allow(dead_code)]

use std::process::Command;
use tracing::{debug, info, warn};

/// `netstat`/`ss`-style lines may list `:30001`, which falsely matches a naive `:3000` substring check.
fn line_has_exact_port_suffix(line: &str, port: u16) -> bool {
    let suffix = format!(":{port}");
    let mut search_from = 0;
    while let Some(rel_idx) = line[search_from..].find(&suffix) {
        let abs = search_from + rel_idx;
        let after = abs + suffix.len();
        let next_is_digit = line
            .get(after..)
            .and_then(|s| s.chars().next())
            .is_some_and(|c| c.is_ascii_digit());
        if !next_is_digit {
            return true;
        }
        search_from = abs + suffix.len();
    }
    false
}

/// Best-effort cleanup for stray legacy gateway processes.
pub fn cleanup_legacy_gateway_processes() -> Result<usize, String> {
    info!("Cleaning up stray legacy agent-diva gateway processes...");
    let mut terminated = 0usize;

    for pid in find_gateway_processes() {
        match terminate_process(pid) {
            Ok(()) => {
                info!("Terminated legacy gateway process {}", pid);
                terminated += 1;
            }
            Err(e) => {
                warn!("Failed to terminate legacy gateway process {}: {}", pid, e);
            }
        }
    }

    info!(
        "Legacy gateway cleanup completed: terminated {} process(es)",
        terminated
    );
    Ok(terminated)
}

/// Forcefully cleans up ALL agent-diva related processes using multiple methods.
#[deprecated(
    note = "Embedded gateway is the default runtime. Keep this only for debug compatibility or cleanup tooling."
)]
pub async fn force_cleanup_all_gateway_processes() -> Result<usize, String> {
    warn!(
        "force_cleanup_all_gateway_processes is deprecated; use embedded mode or cleanup_legacy_gateway_processes for maintenance flows"
    );
    info!("Starting forceful cleanup of all agent-diva gateway processes...");
    let mut terminated = 0;

    // Method 1: Kill process on port 3000 directly
    if let Some(pid) = find_process_on_port_windows_compat() {
        match terminate_process(pid) {
            Ok(()) => {
                info!("Terminated process {} occupying port 3000", pid);
                terminated += 1;
            }
            Err(e) => {
                warn!("Failed to terminate process {} on port 3000: {}", pid, e);
            }
        }
    }

    // Method 2: Kill known agent-diva gateway processes
    for pid in find_gateway_processes() {
        match terminate_process(pid) {
            Ok(()) => {
                info!("Terminated gateway process {}", pid);
                terminated += 1;
            }
            Err(e) => {
                warn!("Failed to terminate gateway process {}: {}", pid, e);
            }
        }
    }

    // Method 3: Try common agent-diva process names
    let process_names = [
        "agent-diva.exe",
        "agent-diva-gui.exe",
        "agent-diva",
        "gateway.exe",
    ];
    for name in process_names {
        if let Ok(pids) = find_processes_by_name(name).await {
            for pid in pids {
                match terminate_process(pid) {
                    Ok(()) => {
                        info!("Terminated {} process {}", name, pid);
                        terminated += 1;
                    }
                    Err(e) => {
                        debug!("Failed to terminate {} process {}: {}", name, pid, e);
                    }
                }
            }
        }
    }

    info!(
        "Force cleanup completed: terminated {} processes total",
        terminated
    );
    Ok(terminated)
}

/// Cleans up orphan agent-diva gateway processes.
#[deprecated(
    note = "Embedded gateway is the default runtime. Keep this only for debug compatibility or cleanup tooling."
)]
pub fn cleanup_orphan_gateway_processes() -> Result<usize, String> {
    warn!(
        "cleanup_orphan_gateway_processes is deprecated; embedded mode does not require orphan-process cleanup during normal startup"
    );
    cleanup_legacy_gateway_processes()
}

/// Checks if a specific port is available for binding
pub fn is_port_available(port: u16) -> bool {
    match std::net::TcpListener::bind(format!("127.0.0.1:{port}")) {
        Ok(_) => {
            debug!("Port {} is available", port);
            true
        }
        Err(e) => {
            debug!("Port {} is not available: {}", port, e);
            false
        }
    }
}

/// Finds the first available port in the given range.
#[deprecated(
    note = "Embedded gateway binds an ephemeral port automatically. This helper remains only for legacy external gateway workflows."
)]
pub fn find_first_available_port(start: u16, end: u16) -> Option<u16> {
    warn!(
        "find_first_available_port is deprecated; embedded gateway uses OS-assigned ephemeral ports"
    );
    info!("Scanning for available ports in range {}-{}", start, end);
    for port in start..=end {
        if is_port_available(port) {
            info!("Found available port: {}", port);
            return Some(port);
        }
    }
    warn!("No available ports found in range {}-{}", start, end);
    None
}

/// Detects if port 3000 is occupied.
#[deprecated(
    note = "Embedded gateway no longer depends on port 3000. This helper remains only for legacy external gateway workflows."
)]
pub fn is_port_3000_occupied() -> bool {
    warn!("is_port_3000_occupied is deprecated; embedded gateway no longer relies on port 3000");
    #[cfg(target_os = "windows")]
    {
        detect_port_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        detect_port_unix()
    }
}

#[cfg(target_os = "windows")]
fn detect_port_windows() -> bool {
    let output = Command::new("netstat").args(["-ano"]).output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let occupied = stdout.lines().any(|line| {
                line_has_exact_port_suffix(line, 3000)
                    && (line.contains("LISTENING") || line.contains("ESTABLISHED"))
            });
            debug!("Port 3000 occupied (Windows): {}", occupied);
            occupied
        }
        Err(e) => {
            warn!("Failed to check port 3000 on Windows: {}", e);
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn detect_port_unix() -> bool {
    // Try lsof first (more reliable)
    let lsof_output = Command::new("lsof").args(["-i", ":3000"]).output();

    if let Ok(output) = lsof_output {
        if !output.stdout.is_empty() {
            debug!("Port 3000 occupied (lsof): true");
            return true;
        }
    }

    // Fallback to netstat
    let netstat_output = Command::new("netstat").args(["-tuln"]).output();

    match netstat_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let occupied = stdout
                .lines()
                .any(|line| line_has_exact_port_suffix(line, 3000));
            debug!("Port 3000 occupied (netstat): {}", occupied);
            occupied
        }
        Err(e) => {
            warn!("Failed to check port 3000 on Unix: {}", e);
            false
        }
    }
}

/// Finds all agent-diva gateway process IDs
pub fn find_gateway_processes() -> Vec<u32> {
    #[cfg(target_os = "windows")]
    {
        find_gateway_processes_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        find_gateway_processes_unix()
    }
}

#[cfg(target_os = "windows")]
fn find_gateway_processes_windows() -> Vec<u32> {
    let mut pids = Vec::new();

    // Try to find agent-diva.exe processes
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq agent-diva.exe", "/FO", "CSV", "/NH"])
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("agent-diva") {
                // Parse CSV format: "name","pid","session","mem","status"
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    if let Ok(pid) = parts[1].trim_matches('"').parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }
    }

    debug!("Found {} agent-diva processes on Windows", pids.len());
    pids
}

#[cfg(not(target_os = "windows"))]
fn find_gateway_processes_unix() -> Vec<u32> {
    let mut pids = Vec::new();

    // Try pgrep first (more reliable)
    let pgrep_output = Command::new("pgrep")
        .args(["-f", "agent-diva.*gateway"])
        .output();

    if let Ok(output) = pgrep_output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                pids.push(pid);
            }
        }
    }

    // Fallback to ps if pgrep didn't find anything
    if pids.is_empty() {
        let ps_output = Command::new("ps").args(["aux"]).output();

        if let Ok(output) = ps_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("agent-diva") && line.contains("gateway") {
                    // Parse ps output to extract PID (second column)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(pid) = parts[1].parse::<u32>() {
                            pids.push(pid);
                        }
                    }
                }
            }
        }
    }

    debug!("Found {} agent-diva gateway processes on Unix", pids.len());
    pids
}

/// Terminates a process by PID
pub fn terminate_process(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        terminate_process_windows(pid)
    }

    #[cfg(not(target_os = "windows"))]
    {
        terminate_process_unix(pid)
    }
}

#[cfg(target_os = "windows")]
fn terminate_process_windows(pid: u32) -> Result<(), String> {
    let output = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to execute taskkill: {}", e))?;

    if output.status.success() {
        debug!("Successfully terminated process {} on Windows", pid);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to terminate process {}: {}", pid, stderr))
    }
}

#[cfg(not(target_os = "windows"))]
fn terminate_process_unix(pid: u32) -> Result<(), String> {
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to execute kill: {}", e))?;

    if output.status.success() {
        debug!("Successfully terminated process {} on Unix", pid);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to terminate process {}: {}", pid, stderr))
    }
}

/// Finds all processes by name
pub async fn find_processes_by_name(name: &str) -> Result<Vec<u32>, String> {
    #[cfg(target_os = "windows")]
    {
        find_processes_by_name_windows(name)
    }

    #[cfg(not(target_os = "windows"))]
    {
        find_processes_by_name_unix(name)
    }
}

#[cfg(target_os = "windows")]
fn find_processes_by_name_windows(name: &str) -> Result<Vec<u32>, String> {
    let mut pids = Vec::new();
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {name}"), "/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| format!("Failed to execute tasklist: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains(name) || line.contains("agent-diva") {
            // Parse CSV format: "name","pid","session","mem","status"
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                if let Ok(pid) = parts[1].trim_matches('"').parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }

    debug!(
        "Found {} process(es) named '{}' on Windows",
        pids.len(),
        name
    );
    Ok(pids)
}

#[cfg(not(target_os = "windows"))]
fn find_processes_by_name_unix(name: &str) -> Result<Vec<u32>, String> {
    let mut pids = Vec::new();
    let pgrep_output = Command::new("pgrep")
        .args(["-f", name])
        .output()
        .map_err(|e| format!("Failed to execute pgrep: {e}"))?;

    let stdout = String::from_utf8_lossy(&pgrep_output.stdout);
    for line in stdout.lines() {
        if let Ok(pid) = line.trim().parse::<u32>() {
            pids.push(pid);
        }
    }

    debug!("Found {} process(es) named '{}' on Unix", pids.len(), name);
    Ok(pids)
}

/// Waits for port 3000 to become available with exponential backoff.
#[deprecated(
    note = "Embedded gateway no longer waits on port 3000. This helper remains only for legacy external gateway workflows."
)]
pub async fn wait_for_port_available(max_attempts: u32, max_wait_ms: u64) -> Result<bool, String> {
    warn!(
        "wait_for_port_available is deprecated; embedded gateway startup no longer waits for port 3000"
    );
    let mut attempt = 0;
    let mut total_waited = 0u64;
    let base_delay_ms = 100u64;

    while attempt < max_attempts {
        if !detect_port_3000_compat() {
            info!(
                "Port 3000 became available after {} attempts ({} ms total)",
                attempt, total_waited
            );
            return Ok(true);
        }

        let delay = base_delay_ms * (2u64.pow(attempt));
        if total_waited + delay > max_wait_ms {
            warn!(
                "Timeout waiting for port 3000: exceeded max_wait_ms={}ms after {}ms",
                max_wait_ms, total_waited
            );
            break;
        }

        debug!(
            "Port 3000 still occupied, waiting {}ms (attempt {}/{})",
            delay,
            attempt + 1,
            max_attempts
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        total_waited += delay;
        attempt += 1;
    }

    warn!(
        "Port 3000 still occupied after {} attempts and {}ms",
        attempt, total_waited
    );
    Ok(false)
}

/// Finds the process ID occupying port 3000.
#[deprecated(
    note = "Embedded gateway no longer depends on port 3000. This helper remains only for legacy external gateway workflows."
)]
pub fn find_process_on_port_3000() -> Option<u32> {
    warn!(
        "find_process_on_port_3000 is deprecated; embedded gateway no longer relies on port 3000"
    );
    find_process_on_port_windows_compat()
}

fn find_process_on_port_windows_compat() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        find_process_on_port_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        find_process_on_port_unix()
    }
}

fn detect_port_3000_compat() -> bool {
    #[cfg(target_os = "windows")]
    {
        detect_port_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        detect_port_unix()
    }
}

#[cfg(target_os = "windows")]
fn find_process_on_port_windows() -> Option<u32> {
    let output = Command::new("netstat").args(["-ano"]).output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line_has_exact_port_suffix(line, 3000)
            && (line.contains("LISTENING") || line.contains("ESTABLISHED"))
        {
            // Extract PID from the last column
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pid_str) = parts.last() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    debug!("Found process {} on port 3000 (Windows)", pid);
                    return Some(pid);
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn find_process_on_port_unix() -> Option<u32> {
    // Try lsof first
    let output = Command::new("lsof")
        .args(["-i", ":3000", "-t"])
        .output()
        .ok()?;

    if !output.stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                debug!("Found process {} on port 3000 (lsof)", pid);
                return Some(pid);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_exact_port_3000_netstat_windows() {
        assert!(line_has_exact_port_suffix(
            "  TCP    127.0.0.1:3000         0.0.0.0:0              LISTENING       1234",
            3000
        ));
    }

    #[test]
    fn line_rejects_30001_as_3000() {
        assert!(!line_has_exact_port_suffix(
            "  TCP    0.0.0.0:30001          0.0.0.0:0              LISTENING       9999",
            3000
        ));
    }

    #[test]
    fn line_rejects_30000_as_3000() {
        assert!(!line_has_exact_port_suffix(
            "  TCP    0.0.0.0:30000         0.0.0.0:0              LISTENING       8888",
            3000
        ));
    }

    #[test]
    fn test_port_detection() {
        // This test just ensures the functions don't panic
        let _ = detect_port_3000_compat();
    }

    #[test]
    fn test_process_finding() {
        // This test just ensures the functions don't panic
        let _ = find_gateway_processes();
    }
}
