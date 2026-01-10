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
mkdir -p "$WORK_DIR" "$WORK_DIR/static"

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