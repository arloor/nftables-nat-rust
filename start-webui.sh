#!/bin/bash
# WebUI 启动示例脚本

# 配置项
PORT=8443
USERNAME="admin"
PASSWORD="your_strong_password_here"
JWT_SECRET=$(openssl rand -base64 32)
CONFIG_FILE="/etc/nat.toml"

# TLS 证书（如果使用 HTTPS）
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
fi

# 启动 WebUI
echo "启动 WebUI..."
echo "访问地址: https://localhost:$PORT"
echo "用户名: $USERNAME"
echo "密码: $PASSWORD"

./target/release/webui \
    --port "$PORT" \
    --username "$USERNAME" \
    --password "$PASSWORD" \
    --jwt-secret "$JWT_SECRET" \
    --toml-config "$CONFIG_FILE" \
    --cert "$CERT_FILE" \
    --key "$KEY_FILE"
