# 必须是root用户
if [ "$(id -u)" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi
# 下载可执行文件
curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v1.0.0/nat -o /tmp/nat
install /tmp/nat /usr/local/bin/nat

# 创建systemd服务
cat > /lib/systemd/system/nat.service <<EOF
[Unit]
Description=nat-service
After=network-online.target
Wants=network-online.target

[Service]
WorkingDirectory=/opt/nat
EnvironmentFile=/opt/nat/env
ExecStart=/usr/local/bin/nat --toml /etc/nat.toml
LimitNOFILE=100000
Restart=always
RestartSec=60

[Install]
WantedBy=multi-user.target
EOF

# 设置开机启动，并启动该服务
systemctl daemon-reload
systemctl enable nat

mkdir /opt/nat
touch /opt/nat/env

# 生成配置文件，配置文件可按需求修改（请看下文）
cat > /etc/nat.toml <<EOF
[[rules]]
type = "single"
sport = 10000
dport = 443
domain = "baidu.com"
protocol = "all"
comment = "This is a comment"

[[rules]]
type = "single"
sport = 10000
dport = 443
domain = "localhost"
protocol = "all"

[[rules]]
type = "range"
portStart = 1000
portEnd = 2000
domain = "baidu.com"
protocol = "tcp"
EOF

systemctl restart nat

echo 编辑 /etc/nat.toml 以自定义规则