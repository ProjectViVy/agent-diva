#!/usr/bin/env bash
# Agent Diva systemd service installer.
# Run from the bundle root or from the systemd/ subdir. Requires root/sudo.
set -euo pipefail

# Resolve bundle root: if we're in systemd/, parent is root; else cwd is root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "$(basename "$SCRIPT_DIR")" == "systemd" ]]; then
    BUNDLE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
else
    BUNDLE_ROOT="$(pwd)"
fi

BINARY_SRC="$BUNDLE_ROOT/bin/agent-diva"
SERVICE_SRC="$SCRIPT_DIR/agent-diva.service"
if [[ ! -f "$SERVICE_SRC" ]]; then
    SERVICE_SRC="$BUNDLE_ROOT/systemd/agent-diva.service"
fi

# Allow override via environment
BIN_DEST="${AGENT_DIVA_BIN:-/usr/bin/agent-diva}"
UNIT_DEST="${AGENT_DIVA_UNIT:-/etc/systemd/system/agent-diva.service}"
DATA_DIR="${AGENT_DIVA_DATA:-/var/lib/agent-diva}"
LOG_DIR="${AGENT_DIVA_LOG:-/var/log/agent-diva}"

# Platform check
if [[ "$(uname -s)" != "Linux" ]]; then
    echo "Error: This installer is for Linux only. Use systemd to manage the service."
    exit 1
fi

if [[ ! -f "$BINARY_SRC" ]]; then
    echo "Error: Binary not found at $BINARY_SRC"
    exit 1
fi

if [[ ! -f "$SERVICE_SRC" ]]; then
    echo "Error: Unit file not found at $SERVICE_SRC"
    exit 1
fi

# Create agent-diva user/group if missing (optional, for strict security)
if ! getent group agent-diva &>/dev/null; then
    echo "Creating group agent-diva..."
    groupadd --system agent-diva
fi
if ! getent passwd agent-diva &>/dev/null; then
    echo "Creating user agent-diva..."
    useradd --system --gid agent-diva --no-create-home --shell /usr/sbin/nologin agent-diva
fi

install -d -m 0755 "$DATA_DIR" "$LOG_DIR"
chown agent-diva:agent-diva "$DATA_DIR" "$LOG_DIR" 2>/dev/null || true

install -m 0755 "$BINARY_SRC" "$BIN_DEST"
install -m 0644 "$SERVICE_SRC" "$UNIT_DEST"

systemctl daemon-reload
systemctl enable agent-diva
systemctl start agent-diva

echo "Agent Diva service installed and started."
echo "  Binary: $BIN_DEST"
echo "  Unit:   $UNIT_DEST"
echo "  Data:   $DATA_DIR"
echo "  Log:    $LOG_DIR"
echo "  Status: $(systemctl is-active agent-diva)"
