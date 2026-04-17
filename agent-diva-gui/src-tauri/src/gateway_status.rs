use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Serializable gateway runtime status for GUI commands and tray integration.
#[derive(Debug, Clone, Serialize)]
pub struct GatewayStatus {
    pub port: u16,
    pub running: bool,
    pub started_at_unix_ms: u128,
}

impl GatewayStatus {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            running: true,
            started_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or(0),
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn format_status(&self) -> String {
        if self.running {
            format!("Gateway: Running (port: {})", self.port)
        } else {
            "Gateway: Stopped".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayStatus;

    #[test]
    fn stop_marks_gateway_as_not_running() {
        let mut status = GatewayStatus::new(4321);
        status.stop();
        assert!(!status.running);
    }
}
