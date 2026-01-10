#!/bin/bash
# NAT 服务安装脚本 - 支持 legacy 和 toml 配置格式

# 使用说明
usage() {
    echo "用法: $0 [legacy|toml]"
    echo "  legacy - 使用传统配置格式 (/etc/nat.conf)"
    echo "  toml   - 使用 TOML 配置格式 (/etc/nat.toml)"
    echo ""
    echo "示例:"
    echo "  $0 legacy"
    echo "  $0 toml"
    exit 1
}

# 检查参数
if [ $# -eq 0 ]; then
    echo "错误: 缺少配置格式参数"
    usage
fi

CONFIG_TYPE="$1"

if [ "$CONFIG_TYPE" != "legacy" ] && [ "$CONFIG_TYPE" != "toml" ]; then
    echo "错误: 无效的配置格式 '$CONFIG_TYPE'"
    usage
fi

# 必须是root用户
if [ "$(id -u)" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi

# 下载可执行文件
echo "下载 nat 可执行文件..."
curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/nat -o /tmp/nat
install /tmp/nat /usr/local/bin/nat

# 根据配置类型设置不同的参数
if [ "$CONFIG_TYPE" = "legacy" ]; then
    EXEC_START="/usr/local/bin/nat /etc/nat.conf"
    CONFIG_FILE="/etc/nat.conf"
    EXAMPLE_FILE="/etc/nat_example.conf"
else
    EXEC_START="/usr/local/bin/nat --toml /etc/nat.toml"
    CONFIG_FILE="/etc/nat.toml"
    EXAMPLE_FILE="/etc/nat_example.toml"
fi

# 创建systemd服务
echo "创建 systemd 服务..."
cat > /lib/systemd/system/nat.service <<EOF
[Unit]
Description=nat-service
After=network-online.target
Wants=network-online.target

[Service]
WorkingDirectory=/opt/nat
EnvironmentFile=/opt/nat/env
ExecStart=$EXEC_START
ExecStop=/bin/bash -c 'nft add table ip self-nat; nft delete table ip self-nat; nft add table ip6 self-nat; nft delete table ip6 self-nat'
LimitNOFILE=100000
Restart=always
RestartSec=60

[Install]
WantedBy=multi-user.target
EOF

# 设置开机启动
systemctl daemon-reload
systemctl enable nat

# 创建工作目录
mkdir -p /opt/nat
touch /opt/nat/env

# 根据配置类型创建配置文件
if [ "$CONFIG_TYPE" = "legacy" ]; then
    echo "创建 legacy 格式配置文件..."
    touch "$CONFIG_FILE"
    
    # 生成示例配置文件
    cat > "$EXAMPLE_FILE" <<EOF
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
EOF
else
    echo "创建 TOML 格式配置文件..."
    # Check if /etc/nat.toml exists, if not create it with example content
    if [ ! -f "$CONFIG_FILE" ]; then
        echo "rules = []" > "$CONFIG_FILE"
        echo "Created $CONFIG_FILE with no rules. Refer to $EXAMPLE_FILE for more example rules."
    fi
    
    # 生成示例配置文件
    cat > "$EXAMPLE_FILE" <<EOF
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
port_start = 1000
port_end = 2000
domain = "baidu.com"
protocol = "tcp"
EOF
fi

# 启动服务
systemctl restart nat

echo ""
echo "========================================="
echo "安装成功，服务已启动！"
echo "========================================="
echo "配置格式: $CONFIG_TYPE"
echo "配置文件: $CONFIG_FILE"
echo "示例配置: $EXAMPLE_FILE"
echo ""
echo "请编辑 $CONFIG_FILE 以自定义规则。"
echo ""
echo "配置示例如下："
echo "----------------------------------------"
cat "$EXAMPLE_FILE"
echo "----------------------------------------"
echo ""
echo "服务管理命令："
echo "  查看状态: systemctl status nat"
echo "  停止服务: systemctl stop nat"
echo "  启动服务: systemctl start nat"
echo "  重启服务: systemctl restart nat"
echo "  查看日志: journalctl -u nat -f"
echo "========================================="
