#!/usr/bin/env bash
# Agent Diva LaunchAgent uninstaller.
# Stops and removes the LaunchAgent. Keeps ~/Library/Logs/agent-diva.
set -euo pipefail

PLIST_DEST="$HOME/Library/LaunchAgents/com.agent-diva.gateway.plist"
LOG_DIR="${AGENT_DIVA_LOG:-$HOME/Library/Logs/agent-diva}"

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Error: This uninstaller is for macOS only."
    exit 1
fi

launchctl unload "$PLIST_DEST" 2>/dev/null || true
rm -f "$PLIST_DEST"

echo "Agent Diva LaunchAgent uninstalled."
echo "  Plist removed."
echo "  Log dir preserved: $LOG_DIR"
echo "  Remove it manually if desired: rm -rf $LOG_DIR"
