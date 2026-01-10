#!/bin/bash
# WebUI 安装和启动脚本

# 使用说明
usage() {
    echo "用法: $0 [PORT]"
    echo "  PORT - WebUI 端口 (默认: 5533)"
    echo ""
    echo "示例:"
    echo "  $0 5533"
    echo "  $0 8444"
    echo ""
    echo "注意:"
    echo "  - 配置格式将自动从现有 NAT 服务配置中检测"
    echo "  - 用户名和密码将在安装过程中交互式输入"
    exit 1
}

# 必须是root用户
if [ "$(id -u)" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi

# 检查 NAT 服务是否已安装
NAT_SERVICE_FILE="/lib/systemd/system/nat.service"
if [ ! -f "$NAT_SERVICE_FILE" ]; then
    echo "错误: 未检测到 NAT 服务"
    echo "请先安装 NAT 服务："
    echo "  TOML 格式: bash <(curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/setup.sh) toml"
    echo "  传统格式: bash <(curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/setup.sh) legacy"
    exit 1
fi

# 从 NAT 服务配置中检测配置格式
echo "检测 NAT 服务配置格式..."
if grep -q "ExecStart.*--toml" "$NAT_SERVICE_FILE"; then
    CONFIG_TYPE="toml"
    echo "检测到 TOML 配置格式"
else
    CONFIG_TYPE="legacy"
    echo "检测到传统配置格式"
fi

echo ""

# 检测系统并安装依赖
echo "检测系统并安装依赖..."
if [ -f /etc/redhat-release ]; then
    # CentOS/RHEL/Fedora
    echo "检测到 RedHat 系系统，使用 yum/dnf 安装依赖..."
    if command -v dnf &> /dev/null; then
        dnf install -y curl openssl
    else
        yum install -y curl openssl
    fi
elif [ -f /etc/debian_version ]; then
    # Debian/Ubuntu
    echo "检测到 Debian 系系统，使用 apt 安装依赖..."
    apt update
    apt install -y curl openssl
else
    echo "警告: 未识别的系统，请手动确保已安装 curl 和 openssl"
fi

# 验证必要工具是否可用
if ! command -v curl &> /dev/null; then
    echo "错误: curl 未安装或不可用"
    exit 1
fi

if ! command -v openssl &> /dev/null; then
    echo "错误: openssl 未安装或不可用"
    exit 1
fi

echo "依赖检查完成"
echo ""

# 从命令行参数获取端口
PORT="${1:-5533}"

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


# 交互式读取用户名和密码
echo ""
echo "========================================="
echo "NAT Console WebUI 配置"
echo "========================================="

# 读取用户名
read -p "请输入登录用户名 [默认: admin]: " USERNAME
USERNAME="${USERNAME:-admin}"

# 读取密码（隐藏输入）
while true; do
    read -s -p "请输入登录密码: " PASSWORD
    echo ""
    if [ -z "$PASSWORD" ]; then
        echo "错误: 密码不能为空，请重新输入"
        continue
    fi
    read -s -p "请再次输入密码确认: " PASSWORD_CONFIRM
    echo ""
    if [ "$PASSWORD" != "$PASSWORD_CONFIRM" ]; then
        echo "错误: 两次输入的密码不一致，请重新输入"
        continue
    fi
    break
done

echo ""
echo "配置信息:"
echo "  配置格式: $CONFIG_TYPE"
echo "  WebUI 端口: $PORT"
echo "  登录用户名: $USERNAME"
echo "========================================="
echo ""

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

systemctl daemon-reload
systemctl enable nat-console
systemctl restart nat-console

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
