//! The host owns the complete sidecar tree before any child code can run.
use std::{
    io,
    process::{Child, Command},
};

pub struct OwnedChild {
    pub child: Child,
    job: Job,
}
impl OwnedChild {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        #[cfg(windows)]
        {
            use std::os::windows::{io::AsRawHandle, process::CommandExt};
            use windows_sys::Win32::{
                Foundation::*,
                System::{JobObjects::*, Threading::*},
            };
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if handle.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let job = Job(handle as usize);
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&info) as u32,
                ) == 0
                {
                    return Err(io::Error::last_os_error());
                }
                let mut child = command
                    .creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW)
                    .spawn()?;
                let result =
                    if AssignProcessToJobObject(handle, child.as_raw_handle() as HANDLE) == 0 {
                        Err(io::Error::last_os_error())
                    } else {
                        resume(child.id())
                    };
                if let Err(e) = result {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(e);
                }
                Ok(Self { child, job })
            }
        }
        #[cfg(not(windows))]
        {
            let _ = command;
            Err(io::Error::other(
                "Diva process ownership is currently verified only on Windows",
            ))
        }
    }
    pub fn kill(&self) {
        self.job.kill();
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.kill();
        let _ = self.child.wait();
    }
}
struct Job(usize);
impl Job {
    fn kill(&self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0 as _, 130);
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0 as _);
        }
    }
}

#[cfg(windows)]
unsafe fn resume(pid: u32) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::*,
        System::{Diagnostics::ToolHelp::*, Threading::*},
    };
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of_val(&entry) as u32;
        let mut found = Thread32First(snapshot, &mut entry);
        let mut result = Err(io::Error::other("suspended core primary thread not found"));
        while found != 0 {
            if entry.th32OwnerProcessID == pid {
                let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                result = if thread.is_null() {
                    Err(io::Error::last_os_error())
                } else {
                    let r = if ResumeThread(thread) == u32::MAX {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(())
                    };
                    CloseHandle(thread);
                    r
                };
                break;
            }
            found = Thread32Next(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        result
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    #[test]
    #[ignore = "subprocess fixture for forced host death"]
    fn owner_fixture() {
        let Some(path) = std::env::var_os("DIVA_JOB_TEST_PIDS") else {
            return;
        };
        let mut cmd = Command::new("python");
        cmd.args(["-u","-c","import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); print(p.pid,flush=True); time.sleep(60)"]).stdout(Stdio::piped());
        let mut owned = OwnedChild::spawn(&mut cmd).unwrap();
        let mut line = String::new();
        BufReader::new(owned.child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        std::fs::write(path, format!("{} {}", owned.child.id(), line.trim())).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(30));
        drop(owned);
    }
    #[test]
    fn forced_host_death_terminates_the_job_tree() {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pids");
        let mut owner = Command::new(std::env::current_exe().unwrap())
            .args(["process::tests::owner_fixture", "--exact", "--ignored"])
            .env("DIVA_JOB_TEST_PIDS", &path)
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        for _ in 0..200 {
            if path.is_file() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let pids = std::fs::read_to_string(path).unwrap();
        let handles: Vec<_> = pids
            .split_whitespace()
            .map(|s| unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, s.parse().unwrap()) })
            .collect();
        owner.kill().unwrap();
        owner.wait().unwrap();
        for handle in handles {
            unsafe {
                assert!(!handle.is_null());
                let result = WaitForSingleObject(handle, 3000);
                CloseHandle(handle);
                assert_eq!(result, 0);
            }
        }
    }
    #[test]
    fn dropping_owner_terminates_descendants() {
        let mut cmd = Command::new("python");
        cmd.args(["-u","-c","import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); print(p.pid,flush=True); time.sleep(60)"]).stdout(Stdio::piped());
        let mut owned = OwnedChild::spawn(&mut cmd).unwrap();
        let mut line = String::new();
        BufReader::new(owned.child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let pid: u32 = line.trim().parse().unwrap();
        unsafe {
            use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};
            let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            assert!(!process.is_null());
            drop(owned);
            assert_eq!(WaitForSingleObject(process, 3000), 0);
            CloseHandle(process);
        }
    }
}
