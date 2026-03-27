// Cross-platform process detection utilities for Gateway process management
// Supports Windows, macOS, and Linux

use std::process::Command;
use tracing::{debug, info, warn};

/// Cleans up orphan agent-diva gateway processes
pub fn cleanup_orphan_gateway_processes() -> Result<usize, String> {
    info!("Scanning for orphan agent-diva gateway processes...");
    let pids = find_gateway_processes();

    if pids.is_empty() {
        info!("No orphan gateway processes found");
        return Ok(0);
    }

    info!(
        "Found {} orphan gateway process(es), terminating...",
        pids.len()
    );
    let mut terminated_count = 0;

    for pid in pids {
        match terminate_process(pid) {
            Ok(()) => {
                info!("Terminated orphan gateway process {}", pid);
                terminated_count += 1;
            }
            Err(e) => {
                warn!("Failed to terminate orphan process {}: {}", pid, e);
            }
        }
    }

    Ok(terminated_count)
}

/// Detects if port 3000 is occupied
pub fn is_port_3000_occupied() -> bool {
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
    let output = Command::new("netstat").args(&["-ano"]).output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let occupied = stdout.lines().any(|line| {
                line.contains(":3000")
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
            let occupied = stdout.lines().any(|line| line.contains(":3000"));
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
        .args(&["/FI", "IMAGENAME eq agent-diva.exe", "/FO", "CSV", "/NH"])
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
        .args(&["/F", "/PID", &pid.to_string()])
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

/// Finds the process ID occupying port 3000
pub fn find_process_on_port_3000() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        find_process_on_port_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        find_process_on_port_unix()
    }
}

#[cfg(target_os = "windows")]
fn find_process_on_port_windows() -> Option<u32> {
    let output = Command::new("netstat").args(&["-ano"]).output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains(":3000") && (line.contains("LISTENING") || line.contains("ESTABLISHED")) {
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
    fn test_port_detection() {
        // This test just ensures the functions don't panic
        let _ = is_port_3000_occupied();
    }

    #[test]
    fn test_process_finding() {
        // This test just ensures the functions don't panic
        let _ = find_gateway_processes();
    }
}
