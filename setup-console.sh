#!/bin/bash
# WebUI 安装和启动脚本

# 使用说明
usage() {
    echo "用法: $0 [CONFIG_TYPE] [PORT] [USERNAME] [PASSWORD]"
    echo "  CONFIG_TYPE - 配置格式: legacy 或 toml (默认: toml)"
    echo "  PORT        - WebUI 端口 (默认: 8444)"
    echo "  USERNAME    - 登录用户名 (默认: admin)"
    echo "  PASSWORD    - 登录密码 (默认: your_strong_password_here)"
    echo ""
    echo "示例:"
    echo "  $0 toml 8444 admin myPassword123"
    echo "  $0 legacy 8444 admin myPassword123"
    exit 1
}

# 从命令行参数获取配置
CONFIG_TYPE="${1:-toml}"
PORT="${2:-8444}"
USERNAME="${3:-admin}"
PASSWORD="${4:-your_strong_password_here}"

# 验证配置类型
if [ "$CONFIG_TYPE" != "legacy" ] && [ "$CONFIG_TYPE" != "toml" ]; then
    echo "错误: 无效的配置格式 '$CONFIG_TYPE'"
    usage
fi

# 检查参数
if [ "$PASSWORD" = "your_strong_password_here" ]; then
    echo "警告: 使用默认密码不安全，请通过参数指定密码"
    usage
fi

# 下载 nat-console
echo "下载 nat-console..."
DOWNLOAD_URL="https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/nat-console"
TMP_FILE="/tmp/nat-console"
INSTALL_PATH="/usr/local/bin/nat-console"

curl -L "$DOWNLOAD_URL" -o "$TMP_FILE"
if [ $? -ne 0 ]; then
    echo "错误: 下载 nat-console 失败"
    exit 1
fi

# 安装到 /usr/local/bin
echo "安装 nat-console 到 $INSTALL_PATH..."
install -m 755 "$TMP_FILE" "$INSTALL_PATH"
if [ $? -ne 0 ]; then
    echo "错误: 安装 nat-console 失败"
    exit 1
fi

echo "nat-console 安装成功"

# 工作目录
WORK_DIR="/opt/nat-console"
sudo mkdir -p "$WORK_DIR" "$WORK_DIR/static"

# 下载静态文件
echo "下载 WebUI 静态文件..."
INDEX_URL="https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/index.html"
LOGIN_URL="https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/login.html"

curl -L "$INDEX_URL" -o "$WORK_DIR/static/index.html"
if [ $? -ne 0 ]; then
    echo "警告: 下载 index.html 失败"
fi

curl -L "$LOGIN_URL" -o "$WORK_DIR/static/login.html"
if [ $? -ne 0 ]; then
    echo "警告: 下载 login.html 失败"
fi

# 配置项
JWT_SECRET=$(openssl rand -base64 32)

# 根据配置类型设置配置文件路径和参数
if [ "$CONFIG_TYPE" = "legacy" ]; then
    CONFIG_FILE="/etc/nat.conf"
    CONFIG_ARG="--compatible-config $CONFIG_FILE"
else
    CONFIG_FILE="/etc/nat.toml"
    CONFIG_ARG="--toml-config $CONFIG_FILE"
fi

# TLS 证书（如果使用 HTTPS）
CERT_FILE="/etc/ssl/nat-webui.crt"
KEY_FILE="/etc/ssl/nat-webui.key"
sudo mkdir -p /etc/ssl

# 如果证书不存在，生成自签名证书（仅用于测试）
if [ ! -f "$CERT_FILE" ] || [ ! -f "$KEY_FILE" ]; then
    echo "生成自签名 TLS 证书..."
    sudo openssl req -x509 -newkey rsa:4096 -nodes \
        -keyout "$KEY_FILE" \
        -out "$CERT_FILE" \
        -days 365 \
        -subj "/CN=localhost"
    sudo chmod 600 "$KEY_FILE"
fi

# 创建 systemd service 文件
echo "创建 systemd service..."
SERVICE_FILE="/lib/systemd/system/nat-console.service"
sudo tee "$SERVICE_FILE" > /dev/null <<EOF
[Unit]
Description=NAT Console WebUI Service
After=network.target

[Service]
Type=simple
WorkingDirectory=$WORK_DIR
ExecStart=$INSTALL_PATH --port $PORT --username $USERNAME --password $PASSWORD --jwt-secret $JWT_SECRET $CONFIG_ARG --cert $CERT_FILE --key $KEY_FILE
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

echo ""
echo "========================================="
echo "安装成功！systemd service 已创建"
echo "========================================="
echo "配置格式: $CONFIG_TYPE"
echo "配置文件: $CONFIG_FILE"
echo "服务文件: $SERVICE_FILE"
echo ""
echo "使用以下命令管理服务:"
echo "  启动服务: sudo systemctl start nat-console"
echo "  停止服务: sudo systemctl stop nat-console"
echo "  查看状态: sudo systemctl status nat-console"
echo "  开机自启: sudo systemctl enable nat-console"
echo "  查看日志: sudo journalctl -u nat-console -f"
echo ""
echo "WebUI 配置:"
echo "  访问地址: https://localhost:$PORT"
echo "  用户名: $USERNAME"
echo "  密码: $PASSWORD"
echo "========================================="
