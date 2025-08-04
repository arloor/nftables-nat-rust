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
ExecStart=/usr/local/bin/nat /etc/nat.conf
ExecStop=/bin/bash -c 'nft add table ip self-nat; nft delete table ip self-nat; nft add table ip6 self-nat; nft delete table ip6 self-nat'
LimitNOFILE=100000
Restart=always
RestartSec=60

[Install]
WantedBy=multi-user.target
EOF

# 设置开机启动，并启动该服务
systemctl daemon-reload
systemctl enable nat

mkdir -p /opt/nat
touch /opt/nat/env
touch /etc/nat.conf

# 生成配置文件，配置文件可按需求修改（请看下文）
cat > /etc/nat_example.conf <<EOF
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
EOF

systemctl restart nat

echo 安装成功，服务已启动。请编辑 /etc/nat.conf 以自定义规则。
echo 配置示例如下：
cat /etc/nat_example.conf
