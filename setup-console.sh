#!/bin/bash
# WebUI 安装和启动脚本

# 使用说明
usage() {
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -p, --port PORT          WebUI 端口 (默认: 5533)"
    echo "  -c, --cert CERT_FILE     TLS 证书文件路径"
    echo "  -k, --key KEY_FILE       TLS 私钥文件路径"
    echo "  -h, --help               显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0                                    # 使用默认端口和自签发证书"
    echo "  $0 -p 8444                            # 指定端口，使用自签发证书"
    echo "  $0 -p 5533 -c /path/cert.pem -k /path/key.pem  # 使用自定义证书"
    echo ""
    echo "注意:"
    echo "  - 配置格式将自动从现有 NAT 服务配置中检测"
    echo "  - 用户名和密码将在安装过程中交互式输入"
    echo "  - 如果未提供证书和私钥，将自动生成自签发证书"
    echo "  - 证书和私钥必须同时提供"
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

# 解析命令行参数
PORT="5533"
USER_CERT_FILE=""
USER_KEY_FILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--port)
            PORT="$2"
            shift 2
            ;;
        -c|--cert)
            USER_CERT_FILE="$2"
            shift 2
            ;;
        -k|--key)
            USER_KEY_FILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "错误: 未知选项 $1"
            usage
            ;;
    esac
done

# 验证证书和私钥参数
if [ -n "$USER_CERT_FILE" ] || [ -n "$USER_KEY_FILE" ]; then
    if [ -z "$USER_CERT_FILE" ] || [ -z "$USER_KEY_FILE" ]; then
        echo "错误: 证书和私钥必须同时提供"
        echo "使用 -c 指定证书，-k 指定私钥"
        exit 1
    fi
    
    if [ ! -f "$USER_CERT_FILE" ]; then
        echo "错误: 证书文件不存在: $USER_CERT_FILE"
        exit 1
    fi
    
    if [ ! -f "$USER_KEY_FILE" ]; then
        echo "错误: 私钥文件不存在: $USER_KEY_FILE"
        exit 1
    fi
fi

bash <(curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/setup-console-assets.sh)

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

# TLS 证书配置
if [ -n "$USER_CERT_FILE" ] && [ -n "$USER_KEY_FILE" ]; then
    # 用户提供了证书和私钥
    CERT_FILE="$USER_CERT_FILE"
    KEY_FILE="$USER_KEY_FILE"
    echo "使用用户提供的 TLS 证书:"
    echo "  证书: $CERT_FILE"
    echo "  私钥: $KEY_FILE"
else
    # 生成自签发证书
    CERT_FILE="/etc/ssl/nat-webui.crt"
    KEY_FILE="/etc/ssl/nat-webui.key"
    mkdir -p /etc/ssl
    
    # 如果证书不存在，生成自签名证书（仅用于测试）
    if [ ! -f "$CERT_FILE" ] || [ ! -f "$KEY_FILE" ]; then
        echo "生成自签名 TLS 证书..."
        openssl req -x509 -newkey rsa:4096 -nodes \
            -keyout "$KEY_FILE" \
            -out "$CERT_FILE" \
            -days 365 \
            -subj "/CN=localhost"
        chmod 600 "$KEY_FILE"
        echo "已生成自签发证书 (仅用于测试环境)"
    else
        echo "使用现有的自签发证书"
    fi
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
tee "$SERVICE_FILE" > /dev/null <<EOF
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
echo "  启动服务: systemctl start nat-console"
echo "  停止服务: systemctl stop nat-console"
echo "  查看状态: systemctl status nat-console"
echo "  开机自启: systemctl enable nat-console"
echo "  查看日志: journalctl -u nat-console -f"
echo ""
echo "WebUI 配置:"
echo "  访问地址: https://localhost:$PORT"
echo "  用户名: $USERNAME"
echo "  密码: $PASSWORD"
echo "========================================="
