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

# 更新现有的 systemd service 文件，移除已废弃的配置文件参数
SERVICE_FILE="/lib/systemd/system/nat-console.service"
if [ -f "$SERVICE_FILE" ]; then
    echo "更新 systemd service 配置..."
    # 移除 --compatible-config 和 --toml-config 参数
    sed -i 's/ --compatible-config [^ ]*//g' "$SERVICE_FILE"
    sed -i 's/ --toml-config [^ ]*//g' "$SERVICE_FILE"
    systemctl daemon-reload
    echo "systemd service 配置已更新，配置格式将自动从 NAT 服务检测"
fi