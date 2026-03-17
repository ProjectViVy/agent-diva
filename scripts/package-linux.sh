#!/bin/bash
#
# Agent Diva Linux 打包脚本
# 在Linux上运行此脚本以创建完整的安装包
#

set -e

VERSION="0.2.0"
PACKAGE_NAME="agent-diva-${VERSION}-linux-x86_64"
DIST_DIR="dist"

# 颜色
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }

# 创建打包目录
info "创建打包目录..."
rm -rf "$DIST_DIR/$PACKAGE_NAME"
mkdir -p "$DIST_DIR/$PACKAGE_NAME"

# 编译
info "编译 release 版本..."
cargo build --release --package agent-diva-cli

# 复制二进制文件
info "复制二进制文件..."
cp target/release/agent-diva "$DIST_DIR/$PACKAGE_NAME/"
chmod +x "$DIST_DIR/$PACKAGE_NAME/agent-diva"

# 复制安装脚本
info "复制安装脚本..."
cp dist/linux/install.sh "$DIST_DIR/$PACKAGE_NAME/"
cp dist/linux/install-offline.sh "$DIST_DIR/$PACKAGE_NAME/"
cp dist/linux/README.md "$DIST_DIR/$PACKAGE_NAME/"
chmod +x "$DIST_DIR/$PACKAGE_NAME/"*.sh

# 创建示例配置
info "创建示例配置..."
mkdir -p "$DIST_DIR/$PACKAGE_NAME/config"
cat > "$DIST_DIR/$PACKAGE_NAME/config/config.json.example" << 'EOF'
{
  "agent": {
    "name": "Diva",
    "default_provider": "openrouter",
    "system_prompt": "You are a helpful AI assistant."
  },
  "providers": {
    "openrouter": {
      "api_key": "YOUR_API_KEY_HERE",
      "base_url": "https://openrouter.ai/api/v1",
      "default_model": "anthropic/claude-3-haiku"
    }
  },
  "channels": {}
}
EOF

# 创建systemd服务文件
info "创建systemd服务文件..."
cat > "$DIST_DIR/$PACKAGE_NAME/agent-diva.service" << 'EOF'
[Unit]
Description=Agent Diva Gateway Service
After=network.target

[Service]
Type=simple
User=%USER%
WorkingDirectory=%HOME%
ExecStart=/usr/local/bin/agent-diva gateway
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# 打包
info "创建压缩包..."
cd "$DIST_DIR"
tar -czvf "${PACKAGE_NAME}.tar.gz" "$PACKAGE_NAME"

# 计算校验和
info "计算校验和..."
sha256sum "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.sha256"

# 清理
rm -rf "$PACKAGE_NAME"

cd ..
success "打包完成: $DIST_DIR/${PACKAGE_NAME}.tar.gz"
success "校验文件: $DIST_DIR/${PACKAGE_NAME}.tar.gz.sha256"

echo ""
echo "文件列表:"
ls -la "$DIST_DIR/"*.tar.gz* 2>/dev/null || true