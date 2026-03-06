#!/usr/bin/env bash
# Agent Diva systemd service uninstaller.
# Stops and removes the service and binary. Keeps /var/lib/agent-diva and /var/log/agent-diva.
set -euo pipefail

BIN_DEST="${AGENT_DIVA_BIN:-/usr/bin/agent-diva}"
UNIT_DEST="${AGENT_DIVA_UNIT:-/etc/systemd/system/agent-diva.service}"
DATA_DIR="${AGENT_DIVA_DATA:-/var/lib/agent-diva}"
LOG_DIR="${AGENT_DIVA_LOG:-/var/log/agent-diva}"

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "Error: This uninstaller is for Linux only."
    exit 1
fi

systemctl stop agent-diva 2>/dev/null || true
systemctl disable agent-diva 2>/dev/null || true
rm -f "$UNIT_DEST"
systemctl daemon-reload

rm -f "$BIN_DEST"

echo "Agent Diva service uninstalled."
echo "  Binary and unit removed."
echo "  Data dir preserved: $DATA_DIR"
echo "  Log dir preserved:  $LOG_DIR"
echo "  Remove them manually if desired: rm -rf $DATA_DIR $LOG_DIR"
