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