#!/bin/bash
#
# Agent Diva macOS 打包脚本
# 用于构建 macOS 通用二进制和 DMG 安装包
#

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

VERSION="0.4.10"
DIST_DIR="dist"

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# 检测是否在 macOS 上
if [[ "$(uname)" != "Darwin" ]]; then
    error "此脚本只能在 macOS 上运行"
fi

# 安装 Rust target
info "检查 Rust targets..."
rustup target add x86_64-apple-darwin 2>/dev/null || true
rustup target add aarch64-apple-darwin 2>/dev/null || true

# 创建输出目录
mkdir -p "$DIST_DIR"

# 编译 x86_64
info "编译 x86_64 (Intel)..."
cargo build --release --package agent-diva-cli --target x86_64-apple-darwin
strip target/x86_64-apple-darwin/release/agent-diva

# 编译 ARM64
info "编译 ARM64 (Apple Silicon)..."
cargo build --release --package agent-diva-cli --target aarch64-apple-darwin
strip target/aarch64-apple-darwin/release/agent-diva

# 创建通用二进制
info "创建通用二进制..."
mkdir -p target/universal/release
lipo -create \
    target/x86_64-apple-darwin/release/agent-diva \
    target/aarch64-apple-darwin/release/agent-diva \
    -output target/universal/release/agent-diva

# 验证通用二进制
info "验证通用二进制..."
lipo -info target/universal/release/agent-diva

# 创建 tarball
info "创建 tarball..."
cp target/universal/release/agent-diva "$DIST_DIR/"
cd "$DIST_DIR"
tar -czvf "agent-diva-${VERSION}-macos-universal.tar.gz" agent-diva

# 检查是否安装了 create-dmg
if command -v create-dmg &> /dev/null; then
    info "创建 DMG 安装包..."
    
    # 创建临时目录
    DMG_TEMP=$(mktemp -d)
    cp target/universal/release/agent-diva "$DMG_TEMP/"
    
    # 创建安装脚本
    cat > "$DMG_TEMP/install.sh" << 'EOF'
#!/bin/bash
echo "Installing Agent Diva..."
sudo cp agent-diva /usr/local/bin/
sudo chmod +x /usr/local/bin/agent-diva
mkdir -p ~/.agent-diva
echo ""
echo "Installation complete!"
echo "Run 'agent-diva --help' to get started."
EOF
    chmod +x "$DMG_TEMP/install.sh"
    
    # 创建 README
    cat > "$DMG_TEMP/README.txt" << EOF
Agent Diva v${VERSION}
====================

Installation:
  1. Open Terminal
  2. cd /Volumes/agent-diva
  3. ./install.sh
  
Or install manually:
  sudo cp agent-diva /usr/local/bin/
  
Usage:
  agent-diva --help
  agent-diva gateway
  agent-diva tui
  
Config directory: ~/.agent-diva
EOF
    
    # 创建 DMG
    create-dmg \
        --volname "Agent Diva ${VERSION}" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        "agent-diva-${VERSION}-macos.dmg" \
        "$DMG_TEMP/"
    
    rm -rf "$DMG_TEMP"
    
    success "DMG 安装包创建完成"
else
    warn "create-dmg 未安装，跳过 DMG 创建"
    info "安装方法: brew install create-dmg"
fi

cd ..
success "打包完成!"
echo ""
echo "产物列表:"
ls -la "$DIST_DIR/"*.tar.gz 2>/dev/null || true
ls -la "$DIST_DIR/"*.dmg 2>/dev/null || true
