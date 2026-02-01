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
ExecStop=/bin/bash -c 'nft add table ip self-nat; nft delete table ip self-nat; nft add table ip6 self-nat; nft delete table ip6 self-nat; nft add table ip self-filter; nft delete table ip self-filter; nft add table ip6 self-filter; nft delete table ip6 self-filter'
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
    if [ ! -s "$CONFIG_FILE" ]; then
        cat > "$CONFIG_FILE" <<EOF
# 配置方式参考 https://github.com/arloor/nftables-nat-rust/blob/master/README.md#%E4%BC%A0%E7%BB%9F%E9%85%8D%E7%BD%AE%E6%96%87%E4%BB%B6
EOF
    fi
    
    # 生成示例配置文件
    cat > "$EXAMPLE_FILE" <<EOF
# 单端口转发：本机端口 -> 目标地址:端口
SINGLE,49999,59999,example.com
# 端口段转发：本机端口段 -> 目标地址:端口段
RANGE,50000,50010,example.com
# 端口重定向：外部端口 -> 本机端口
REDIRECT,8000,3128
# 端口段重定向：外部端口段 -> 本机端口
REDIRECT,30001-39999,45678
# 仅转发 TCP 流量
SINGLE,10000,443,example.com,tcp
# 仅转发 UDP 流量
SINGLE,10001,53,dns.example.com,udp
# 以 # 开头的行为注释
# SINGLE,3000,3000,disabled.example.com
EOF
else
    echo "创建 TOML 格式配置文件..."
    # Check if /etc/nat.toml exists, if not create it with example content
    if [ ! -s "$CONFIG_FILE" ]; then
        cat > "$CONFIG_FILE" <<EOF
# 配置方式参考 https://github.com/arloor/nftables-nat-rust/blob/master/README.md#toml-%E9%85%8D%E7%BD%AE%E6%96%87%E4%BB%B6%E6%8E%A8%E8%8D%90
rules = []
EOF
    fi
    
    # 生成示例配置文件
    cat > "$EXAMPLE_FILE" <<EOF
# 单端口转发示例
[[rules]]
type = "single"
sport = 10000          # 本机端口
dport = 443            # 目标端口
domain = "example.com" # 目标域名或 IP
protocol = "all"       # all, tcp 或 udp
ip_version = "ipv4"    # ipv4, ipv6 或 all
comment = "HTTPS 转发"

# 端口段转发示例
[[rules]]
type = "range"
port_start = 20000      # 起始端口
port_end = 20100        # 结束端口
domain = "example.com"
protocol = "tcp"
ip_version = "all"    # 同时支持 IPv4 和 IPv6
comment = "端口段转发"

# 单端口重定向示例
[[rules]]
type = "redirect"
sport = 8080         # 源端口
dport = 3128         # 目标端口
protocol = "all"
ip_version = "ipv4"
comment = "单端口重定向到本机"

# 端口段重定向示例
[[rules]]
type = "redirect"
sport = 30001        # 起始端口
sport_end = 39999     # 结束端口
dport = 45678        # 目标端口
protocol = "tcp"
ip_version = "all"
comment = "端口段重定向到本机"

# 强制 IPv6 转发
[[rules]]
type = "single"
sport = 9001
dport = 9090
domain = "ipv6.example.com"
protocol = "all"
ip_version = "ipv6"    # 仅使用 IPv6
comment = "IPv6 专用转发"
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
