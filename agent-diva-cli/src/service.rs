use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone)]
#[command(rename_all = "kebab-case")]
pub enum ServiceCommands {
    /// Install or update the Windows service definition
    Install {
        /// Configure the service to auto-start with Windows
        #[arg(long)]
        auto_start: bool,
        /// Print planned SCM operations without executing (for CI validation)
        #[arg(long)]
        dry_run: bool,
    },
    /// Start the installed service
    Start {
        /// Print planned SCM operations without executing (for CI validation)
        #[arg(long)]
        dry_run: bool,
    },
    /// Stop the running service
    Stop {
        /// Print planned SCM operations without executing (for CI validation)
        #[arg(long)]
        dry_run: bool,
    },
    /// Restart the service
    Restart {
        /// Print planned SCM operations without executing (for CI validation)
        #[arg(long)]
        dry_run: bool,
    },
    /// Uninstall the service
    Uninstall {
        /// Print planned SCM operations without executing (for CI validation)
        #[arg(long)]
        dry_run: bool,
    },
    /// Show the current service status
    Status {
        /// Output status as JSON for automation
        #[arg(long)]
        json: bool,
        /// Print planned SCM operations without executing (for CI validation)
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn run_service_command(
    config_dir: Option<&PathBuf>,
    command: ServiceCommands,
) -> Result<()> {
    #[cfg(windows)]
    {
        windows_impl::run_service_command(config_dir, command)
    }

    #[cfg(not(windows))]
    {
        let _ = config_dir;
        let _ = command;
        anyhow::bail!("service subcommands are only supported on Windows");
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::ServiceCommands;
    use anyhow::{Context, Result};
    use console::style;
    use serde::Serialize;
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};
    use windows_service::service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    };
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    const SERVICE_NAME: &str = "AgentDivaGateway";
    const SERVICE_DISPLAY_NAME: &str = "Agent Diva Gateway Service";
    const SERVICE_DESCRIPTION: &str = "Runs Agent Diva gateway as a background Windows service.";

    #[derive(Debug, Serialize)]
    struct ServiceStatusOutput {
        installed: bool,
        running: bool,
        state: String,
        executable_path: Option<String>,
    }

    fn sibling_service_binary(current_exe: &Path) -> PathBuf {
        current_exe
            .parent()
            .map(|dir| dir.join("agent-diva-service.exe"))
            .unwrap_or_else(|| PathBuf::from("agent-diva-service.exe"))
    }

    pub fn run_service_command(
        config_dir: Option<&PathBuf>,
        command: ServiceCommands,
    ) -> Result<()> {
        match command {
            ServiceCommands::Install {
                auto_start,
                dry_run,
            } => install_service(config_dir, auto_start, dry_run),
            ServiceCommands::Start { dry_run } => start_service(dry_run),
            ServiceCommands::Stop { dry_run } => stop_service(dry_run),
            ServiceCommands::Restart { dry_run } => {
                stop_service(dry_run)?;
                start_service(dry_run)
            }
            ServiceCommands::Uninstall { dry_run } => uninstall_service(dry_run),
            ServiceCommands::Status { json, dry_run } => status_service(json, dry_run),
        }
    }

    fn dry_run_print(msg: &str) {
        println!("[dry-run] {}", msg);
    }

    fn service_manager(access: ServiceManagerAccess) -> Result<ServiceManager> {
        ServiceManager::local_computer(None::<&str>, access)
            .context("failed to connect to Windows Service Manager")
    }

    fn service_binary_path() -> Result<PathBuf> {
        let current_exe = std::env::current_exe().context("failed to resolve current CLI path")?;
        let service_exe = sibling_service_binary(&current_exe);
        if !service_exe.exists() {
            anyhow::bail!(
                "service binary not found at {}. Build agent-diva-service first.",
                service_exe.display()
            );
        }
        Ok(service_exe)
    }

    fn launch_arguments(config_dir: Option<&PathBuf>) -> Vec<OsString> {
        let mut args = Vec::new();
        if let Some(config_dir) = config_dir {
            args.push(OsString::from("--config-dir"));
            args.push(config_dir.as_os_str().to_os_string());
        }
        args
    }

    fn desired_service_info(config_dir: Option<&PathBuf>, auto_start: bool) -> Result<ServiceInfo> {
        Ok(ServiceInfo {
            name: OsString::from(SERVICE_NAME),
            display_name: OsString::from(SERVICE_DISPLAY_NAME),
            service_type: ServiceType::OWN_PROCESS,
            start_type: if auto_start {
                ServiceStartType::AutoStart
            } else {
                ServiceStartType::OnDemand
            },
            error_control: ServiceErrorControl::Normal,
            executable_path: service_binary_path()?,
            launch_arguments: launch_arguments(config_dir),
            dependencies: vec![],
            account_name: None,
            account_password: None,
        })
    }

    fn install_service(
        config_dir: Option<&PathBuf>,
        auto_start: bool,
        dry_run: bool,
    ) -> Result<()> {
        if dry_run {
            let service_info = desired_service_info(config_dir, auto_start)?;
            dry_run_print(&format!(
                "would install service {} with executable_path={}, auto_start={}",
                SERVICE_NAME,
                service_info.executable_path.display(),
                auto_start
            ));
            return Ok(());
        }
        let manager =
            service_manager(ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)?;
        let service_info = desired_service_info(config_dir, auto_start)?;

        let access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::START
            | ServiceAccess::STOP
            | ServiceAccess::CHANGE_CONFIG
            | ServiceAccess::DELETE;

        let service = match manager.open_service(SERVICE_NAME, access) {
            Ok(service) => {
                service
                    .change_config(&service_info)
                    .context("failed to update existing service configuration")?;
                println!(
                    "{} Updated Windows service definition",
                    style("[OK]").green().bold()
                );
                service
            }
            Err(_) => {
                let service = manager
                    .create_service(&service_info, access)
                    .context("failed to create Windows service")?;
                println!("{} Installed Windows service", style("[OK]").green().bold());
                service
            }
        };

        service
            .set_description(SERVICE_DESCRIPTION)
            .context("failed to set service description")?;

        if auto_start {
            service
                .set_delayed_auto_start(true)
                .context("failed to enable delayed auto-start")?;
        }

        println!("Service binary: {}", service_info.executable_path.display());
        Ok(())
    }

    fn start_service(dry_run: bool) -> Result<()> {
        if dry_run {
            dry_run_print(&format!("would start service {}", SERVICE_NAME));
            return Ok(());
        }
        let manager = service_manager(ServiceManagerAccess::CONNECT)?;
        let service = manager
            .open_service(
                SERVICE_NAME,
                ServiceAccess::START | ServiceAccess::QUERY_STATUS,
            )
            .context("failed to open service for start")?;

        let status = service
            .query_status()
            .context("failed to query service status")?;
        if status.current_state == ServiceState::Running {
            println!(
                "{} Service is already running",
                style("[OK]").green().bold()
            );
            return Ok(());
        }

        service
            .start(&[] as &[&OsStr])
            .context("failed to start Windows service")?;
        wait_for_state(&service, ServiceState::Running, Duration::from_secs(30))?;
        println!("{} Service started", style("[OK]").green().bold());
        Ok(())
    }

    fn stop_service(dry_run: bool) -> Result<()> {
        if dry_run {
            dry_run_print(&format!("would stop service {}", SERVICE_NAME));
            return Ok(());
        }
        let manager = service_manager(ServiceManagerAccess::CONNECT)?;
        let service = manager
            .open_service(
                SERVICE_NAME,
                ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
            )
            .context("failed to open service for stop")?;

        let status = service
            .query_status()
            .context("failed to query service status")?;
        if status.current_state == ServiceState::Stopped {
            println!(
                "{} Service is already stopped",
                style("[OK]").green().bold()
            );
            return Ok(());
        }

        service.stop().context("failed to stop Windows service")?;
        wait_for_state(&service, ServiceState::Stopped, Duration::from_secs(30))?;
        println!("{} Service stopped", style("[OK]").green().bold());
        Ok(())
    }

    fn uninstall_service(dry_run: bool) -> Result<()> {
        if dry_run {
            dry_run_print(&format!("would uninstall service {}", SERVICE_NAME));
            return Ok(());
        }
        let manager = service_manager(ServiceManagerAccess::CONNECT)?;
        let service = manager
            .open_service(
                SERVICE_NAME,
                ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
            )
            .context("failed to open service for uninstall")?;

        let status = service
            .query_status()
            .context("failed to query service status")?;
        if status.current_state != ServiceState::Stopped {
            service
                .stop()
                .context("failed to stop service before uninstall")?;
            wait_for_state(&service, ServiceState::Stopped, Duration::from_secs(30))?;
        }

        service.delete().context("failed to delete service")?;
        println!("{} Service uninstalled", style("[OK]").green().bold());
        Ok(())
    }

    fn status_service(json: bool, dry_run: bool) -> Result<()> {
        if dry_run {
            dry_run_print(&format!("would query status for service {}", SERVICE_NAME));
            return Ok(());
        }
        let manager = service_manager(ServiceManagerAccess::CONNECT)?;
        let output = match manager.open_service(
            SERVICE_NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::QUERY_CONFIG,
        ) {
            Ok(service) => {
                let status = service
                    .query_status()
                    .context("failed to query service status")?;
                let config = service
                    .query_config()
                    .context("failed to query service config")?;
                ServiceStatusOutput {
                    installed: true,
                    running: status.current_state == ServiceState::Running,
                    state: format!("{:?}", status.current_state),
                    executable_path: Some(config.executable_path.display().to_string()),
                }
            }
            Err(_) => ServiceStatusOutput {
                installed: false,
                running: false,
                state: "NotInstalled".to_string(),
                executable_path: None,
            },
        };

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .context("failed to serialize service status as JSON")?
            );
        } else if output.installed {
            println!("Service: {}", SERVICE_NAME);
            println!("Installed: yes");
            println!("Running: {}", if output.running { "yes" } else { "no" });
            println!("State: {}", output.state);
            if let Some(path) = output.executable_path {
                println!("Executable: {}", path);
            }
        } else {
            println!("Service: {}", SERVICE_NAME);
            println!("Installed: no");
            println!("Running: no");
            println!("State: NotInstalled");
        }

        Ok(())
    }

    fn wait_for_state(
        service: &windows_service::service::Service,
        desired_state: ServiceState,
        timeout: Duration,
    ) -> Result<()> {
        let started_at = Instant::now();
        loop {
            let status = service
                .query_status()
                .context("failed to query service status")?;
            if status.current_state == desired_state {
                return Ok(());
            }
            if started_at.elapsed() >= timeout {
                anyhow::bail!(
                    "timed out waiting for service to reach state {:?}; current state is {:?}",
                    desired_state,
                    status.current_state
                );
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}
