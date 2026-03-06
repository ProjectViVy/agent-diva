#!/usr/bin/env bash
# Agent Diva LaunchAgent installer.
# Run from the bundle root or from the launchd/ subdir. User-level install (no sudo).
set -euo pipefail

# Resolve bundle root: if we're in launchd/, parent is root; else cwd is root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "$(basename "$SCRIPT_DIR")" == "launchd" ]]; then
    BUNDLE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
else
    BUNDLE_ROOT="$(pwd)"
fi

# Prefer bin/agent-diva (Headless package), fallback to bin/macos/agent-diva (GUI bundle)
if [[ -f "$BUNDLE_ROOT/bin/agent-diva" ]]; then
    BINARY_SRC="$BUNDLE_ROOT/bin/agent-diva"
elif [[ -f "$BUNDLE_ROOT/bin/macos/agent-diva" ]]; then
    BINARY_SRC="$BUNDLE_ROOT/bin/macos/agent-diva"
else
    BINARY_SRC="$BUNDLE_ROOT/bin/agent-diva"
fi
PLIST_SRC="$SCRIPT_DIR/com.agent-diva.gateway.plist"
if [[ ! -f "$PLIST_SRC" ]]; then
    PLIST_SRC="$BUNDLE_ROOT/launchd/com.agent-diva.gateway.plist"
fi

# Allow override via environment
BINARY_PATH="${AGENT_DIVA_BIN:-$BINARY_SRC}"
LOG_DIR="${AGENT_DIVA_LOG:-$HOME/Library/Logs/agent-diva}"
PLIST_DEST="$HOME/Library/LaunchAgents/com.agent-diva.gateway.plist"

# Platform check
if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Error: This installer is for macOS only. Use launchctl to manage the service."
    exit 1
fi

if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

if [[ ! -f "$PLIST_SRC" ]]; then
    echo "Error: Plist template not found at $PLIST_SRC"
    exit 1
fi

# Resolve absolute paths for plist
BINARY_ABS="$(realpath "$BINARY_PATH" 2>/dev/null || (cd "$(dirname "$BINARY_PATH")" && pwd)/$(basename "$BINARY_PATH"))"
LOG_DIR_ABS="${LOG_DIR/#\~/$HOME}"
mkdir -p "$LOG_DIR_ABS"

# Generate plist with substituted paths
sed -e "s|__BINARY_PATH__|$BINARY_ABS|g" \
    -e "s|__LOG_DIR__|$LOG_DIR_ABS|g" \
    "$PLIST_SRC" > "$PLIST_DEST"

# Load the LaunchAgent
launchctl unload "$PLIST_DEST" 2>/dev/null || true
launchctl load "$PLIST_DEST"

echo "Agent Diva LaunchAgent installed and loaded."
echo "  Binary:  $BINARY_ABS"
echo "  Plist:   $PLIST_DEST"
echo "  Log dir: $LOG_DIR_ABS"
echo "  Status:  $(launchctl list | grep com.agent-diva.gateway || echo 'loaded')"
