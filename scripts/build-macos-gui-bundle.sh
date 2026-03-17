#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "[1/3] 构建 CLI（二进制 agent-diva）..."
cd "$ROOT_DIR"
cargo build --release -p agent-diva-cli

echo "[2/3] 为 macOS 准备 GUI 资源（内置 CLI）..."
python3 scripts/ci/prepare_gui_bundle.py \
  --gui-root agent-diva-gui \
  --target-os macos

echo "[3/3] 构建 Tauri GUI（.app / .dmg）..."
cd "$ROOT_DIR/agent-diva-gui"
pnpm install
pnpm tauri build

echo
echo "✅ 构建完成，dmg 位置大致为："
echo "  $ROOT_DIR/agent-diva-gui/src-tauri/target/release/bundle/dmg"

