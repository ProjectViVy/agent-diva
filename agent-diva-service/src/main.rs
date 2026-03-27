use clap::Parser;
#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;

#[cfg(windows)]
const SERVICE_NAME: &str = "AgentDivaGateway";

#[derive(Parser, Debug)]
#[command(name = "agent-diva-service")]
#[command(about = "Windows service companion for Agent Diva")]
struct Cli {
    /// Optional configuration directory forwarded to `agent-diva gateway run`
    #[arg(long)]
    config_dir: Option<PathBuf>,

    /// Run the service companion in console mode for local validation.
    #[arg(long)]
    console: bool,
}

#[cfg(windows)]
fn sibling_cli_path(current_exe: &Path) -> PathBuf {
    current_exe
        .parent()
        .map(|dir| dir.join("agent-diva.exe"))
        .unwrap_or_else(|| PathBuf::from("agent-diva.exe"))
}

#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    let _ = Cli::parse();
    anyhow::bail!("agent-diva-service is only supported on Windows")
}

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    windows_impl::run(cli)
}

#[cfg(windows)]
mod windows_impl {
    use super::{sibling_cli_path, Cli, SERVICE_NAME};
    use anyhow::{Context, Result};
    use clap::Parser;
    use std::ffi::OsString;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;
    use tracing::{error, info, warn};
    use windows_service::define_windows_service;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_dispatcher;

    define_windows_service!(ffi_service_main, service_main);

    pub fn run(cli: Cli) -> Result<()> {
        init_logging();

        if cli.console {
            return run_console(cli);
        }

        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
            .context("failed to start Windows service dispatcher")
    }

    fn init_logging() {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with_target(false)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    fn run_console(cli: Cli) -> Result<()> {
        let current_exe =
            std::env::current_exe().context("failed to resolve service executable")?;
        let cli_path = sibling_cli_path(&current_exe);
        let mut child =
            spawn_gateway_child(&cli_path, cli.config_dir.as_ref()).with_context(|| {
                format!("failed to launch gateway child via {}", cli_path.display())
            })?;
        wait_for_child_exit(&mut child)
    }

    fn service_main(arguments: Vec<OsString>) {
        let cli =
            Cli::parse_from(std::iter::once(OsString::from("agent-diva-service")).chain(arguments));

        if let Err(error) = run_service(cli) {
            error!("service exited with error: {error:#}");
        }
    }

    fn run_service(cli: Cli) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        let status_handle =
            service_control_handler::register(
                SERVICE_NAME,
                move |control_event| match control_event {
                    ServiceControl::Stop | ServiceControl::Shutdown => {
                        let _ = shutdown_tx.send(());
                        ServiceControlHandlerResult::NoError
                    }
                    ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                    _ => ServiceControlHandlerResult::NotImplemented,
                },
            )
            .context("failed to register service control handler")?;

        status_handle
            .set_service_status(service_status(ServiceState::StartPending))
            .context("failed to report StartPending")?;

        let current_exe =
            std::env::current_exe().context("failed to resolve service executable")?;
        let cli_path = sibling_cli_path(&current_exe);
        let mut child =
            spawn_gateway_child(&cli_path, cli.config_dir.as_ref()).with_context(|| {
                format!("failed to launch gateway child via {}", cli_path.display())
            })?;

        status_handle
            .set_service_status(service_status(ServiceState::Running))
            .context("failed to report Running")?;

        let wait_result = wait_for_stop_or_child(&mut child, shutdown_rx);

        status_handle
            .set_service_status(service_status(ServiceState::StopPending))
            .context("failed to report StopPending")?;

        stop_child(&mut child);

        status_handle
            .set_service_status(service_status(ServiceState::Stopped))
            .context("failed to report Stopped")?;

        wait_result
    }

    fn service_status(current_state: ServiceState) -> ServiceStatus {
        ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state,
            controls_accepted: if current_state == ServiceState::Running {
                ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
            } else {
                ServiceControlAccept::empty()
            },
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        }
    }

    fn spawn_gateway_child(
        cli_path: &Path,
        config_dir: Option<&std::path::PathBuf>,
    ) -> Result<Child> {
        let mut command = Command::new(cli_path);
        command
            .arg("gateway")
            .arg("run")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        if let Some(config_dir) = config_dir {
            command.arg("--config-dir").arg(config_dir);
        }

        command.spawn().context("failed to spawn gateway child")
    }

    fn wait_for_child_exit(child: &mut Child) -> Result<()> {
        let status = child.wait().context("failed to wait for gateway child")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("gateway child exited with status {status}")
        }
    }

    fn wait_for_stop_or_child(child: &mut Child, shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        loop {
            if let Some(status) = child.try_wait().context("failed to poll gateway child")? {
                if status.success() {
                    return Ok(());
                }
                anyhow::bail!("gateway child exited unexpectedly with status {status}");
            }

            match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(()) => {
                    info!("received service stop signal");
                    return Ok(());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    warn!("service shutdown channel disconnected; stopping child");
                    return Ok(());
                }
            }
        }
    }

    fn stop_child(child: &mut Child) {
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                if let Err(error) = child.kill() {
                    warn!("failed to terminate gateway child: {error}");
                }
                if let Err(error) = child.wait() {
                    warn!("failed to wait for gateway child shutdown: {error}");
                }
            }
            Err(error) => warn!("failed to poll child before shutdown: {error}"),
        }
    }
}
