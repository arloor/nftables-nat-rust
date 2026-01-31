# NFTables NAT Rust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

基于 nftables 的高性能 NAT 端口转发管理工具，使用 Rust 语言开发。

## ✨ 核心特性

- 🔄 **动态 NAT 转发**：自动监测配置文件和目标域名 IP 变化，实时更新转发规则
- 🌐 **IPv4/IPv6 双栈支持**：完整支持 IPv4 和 IPv6 NAT 转发
- 📝 **灵活配置**：支持传统配置文件和 TOML 格式，满足不同使用场景
- 🎯 **精准控制**：支持单端口、端口段、TCP/UDP 协议选择
- 🔌 **本地重定向**：支持端口重定向到本机其他端口
- 🐋 **Docker 兼容**：与 Docker 网络完美兼容
- ⚡ **高性能轻量**：基于 Rust 编写，仅依赖标准库和少量核心库
- 🚀 **开机自启**：支持 systemd 服务管理，开机自动启动
- 🔍 **域名解析**：支持域名和 IP 地址，自动 DNS 解析和缓存
- 🖥️ **Web 管理界面**：提供可视化的 WebUI 管理配置和查看规则，并且支持切换后端地址

## 🖥️ 系统要求

适用于以下 Linux 发行版：

- CentOS 8+ / RHEL 8+ / Fedora
- Debian 10+ / Ubuntu 18.04+
- 其他支持 nftables 的现代 Linux 发行版

## ⚙️ 系统准备

### CentOS / RHEL / Fedora

```bash
# 关闭 firewalld
systemctl disable --now firewalld

# 关闭 SELinux
setenforce 0
sed -i 's/SELINUX=enforcing/SELINUX=disabled/' /etc/selinux/config

# 安装 nftables
yum install -y nftables
```

### Debian / Ubuntu

```bash
# 安装 nftables
apt update && apt install -y nftables

# 禁用 iptables（可选）
systemctl disable --now iptables
```

## 📦 快速安装

> 升级也使用相同的安装命令

### 方法一：TOML 配置文件版本（推荐）

```bash
bash <(curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/setup.sh) toml
```

### 方法二：传统配置文件版本

```bash
bash <(curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/setup.sh) legacy
```

## 🆕 WebUI 管理界面

本项目现已支持 Web 管理界面，可以通过浏览器方便地管理 NAT 配置。

- 🔐 基于 JWT 的安全认证
- 🔒 支持 HTTPS/TLS 加密传输
- 📝 可视化编辑配置文件（支持传统格式和 TOML 格式）
- 📋 实时查看 nftables 规则
- 🌐 支持多后端地址切换，可管理多台服务器
- 🎨 现代化的用户界面

### 安装管理界面 WebUI

```bash
bash <(curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/setup-console.sh) # -p 5533  -k /root/.acme.sh/arloor.dev/arloor.dev.key -c /root/.acme.sh/arloor.dev/fullchain.cer
```

1. 安装过程会交互式提示输入用户名和密码。密码会保存在 systemd 文件中，注意安全。
2. 通过 `-p` 参数可以指定 WebUI 监听端口，默认端口为 5533。
3. 通过 `-c` 和 `-k` 参数可以指定自定义 TLS 证书和私钥文件路径，如果未提供，将自动生成自签名证书。
4. 安装脚本会自动检测现有 NAT 服务的配置格式，并根据配置格式生成相应的 systemd service 文件。

安装完成后，访问 `https://your-server-ip:5533` 即可使用管理界面。

**多后端管理**：登录页面可配置后端 API 地址，支持跨域访问不同服务器。在"后端设置"标签页可添加、切换多个后端地址，方便管理多台服务器。留空后端地址则使用当前服务器。

详细文档请查看 [nat-console/README.md](nat-console/README.md)

### 升级 WebUI

```bash
bash <(curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v2.0.0/setup-console-assets.sh)
systemctl restart nat-console
```

### WebUI 页面

![alt text](image.png)

![alt text](image1.png)

## 📝 配置说明

### TOML 配置文件（推荐）

配置文件位置：`/etc/nat.toml`

**优势**：

- ✅ 支持配置验证，保证格式正确
- ✅ 支持注释，便于维护
- ✅ WebUI 可视化编辑和验证
- ✅ 结构化配置，可读性更好

```toml
# ============ 基础转发示例 ============

# 1. 单端口转发 - HTTPS 流量转发
[[rules]]
type = "single"
sport = 10443          # 本机监听端口
dport = 443            # 目标服务端口
domain = "example.com" # 目标域名或 IP 地址
protocol = "all"       # 协议: all, tcp 或 udp
ip_version = "ipv4"    # IP 版本: ipv4, ipv6 或 all
comment = "转发 HTTPS 到 example.com"

# 2. 端口段转发 - 批量游戏端口
[[rules]]
type = "range"
port_start = 20000     # 起始端口
port_end = 20100       # 结束端口（含）
domain = "game.example.com"
protocol = "tcp"       # 仅 TCP 协议
ip_version = "all"     # 同时支持 IPv4 和 IPv6
comment = "游戏服务器端口段"

# 3. UDP 专用转发 - DNS 服务
[[rules]]
type = "single"
sport = 5353           # 本机 DNS 端口
dport = 53             # 目标 DNS 端口
domain = "8.8.8.8"     # 也可以直接使用 IP 地址
protocol = "udp"       # 仅 UDP 协议
ip_version = "ipv4"
comment = "DNS 查询转发"

# ============ 本地重定向示例 ============

# 4. 单端口重定向到本机服务
[[rules]]
type = "redirect"
sport = 8080           # 外部访问端口
dport = 3128           # 本机实际服务端口
protocol = "all"
ip_version = "ipv4"
comment = "代理服务端口重定向"

# 5. 端口段重定向到本机
[[rules]]
type = "redirect"
sport = 30001          # 起始端口
sport_end = 30100      # 结束端口
dport = 45678          # 本机目标端口
protocol = "tcp"
ip_version = "all"
comment = "批量端口重定向到本机"

# ============ 高级场景示例 ============

# 6. 强制 IPv6 转发
[[rules]]
type = "single"
sport = 9001
dport = 9090
domain = "ipv6.example.com"
protocol = "all"
ip_version = "ipv6"    # 仅使用 IPv6 进行转发
comment = "IPv6 专用服务"

# 7. 双栈支持示例 - 自动选择 IPv4/IPv6
[[rules]]
type = "single"
sport = 10080
dport = 80
domain = "dual-stack.example.com"  # 域名同时有 A 和 AAAA 记录
protocol = "tcp"
ip_version = "all"     # 根据客户端 IP 版本自动选择
comment = "双栈 Web 服务"
```

### 传统配置文件

配置文件位置：`/etc/nat.conf`

**基础格式**：

- `SINGLE,本机端口,目标端口,目标地址[,协议][,IP版本]` - 单端口转发
- `RANGE,起始端口,结束端口,目标地址[,协议][,IP版本]` - 端口段转发
- `REDIRECT,源端口,目标端口[,协议][,IP版本]` - 重定向到本机端口
- `REDIRECT,起始端口-结束端口,目标端口[,协议][,IP版本]` - 端口段重定向

**参数说明**：

- 协议可选值：`tcp`、`udp`、`all`（默认为 `all`）
- IP 版本可选值：`ipv4`、`ipv6`、`all`（默认为 `all`）
- 以 `#` 开头的行为注释

**配置示例**：

```bash
# ============ 基础转发 ============

# 单端口转发 - HTTPS 流量
SINGLE,10443,443,example.com

# 端口段转发 - 游戏服务器端口（20000-20100）
RANGE,20000,20100,game.example.com

# ============ 协议指定 ============

# 仅转发 TCP 流量 - Web 服务
SINGLE,10080,80,web.example.com,tcp

# 仅转发 UDP 流量 - DNS 查询
SINGLE,5353,53,8.8.8.8,udp

# ============ 本地重定向 ============

# 单端口重定向到本机服务
REDIRECT,8080,3128

# 端口段重定向到本机（30001-30100 → 45678）
REDIRECT,30001-30100,45678

# TCP 专用重定向
REDIRECT,7000-7100,8080,tcp

# ============ IPv6 支持 ============

# 强制使用 IPv6 转发
SINGLE,9001,9090,ipv6.example.com,all,ipv6

# 双栈支持（根据客户端自动选择）
SINGLE,10080,80,dual-stack.example.com,tcp,all

# 禁用的规则（以 # 开头）
# SINGLE,3000,3000,disabled.example.com
```

## 🚀 使用方法

### 启动/停止服务

```bash
# 启动服务
systemctl start nat

# 停止服务
systemctl stop nat

# 重启服务
systemctl restart nat

# 查看服务状态
systemctl status nat

# 开机自启
systemctl enable nat

# 取消开机自启
systemctl disable nat
```

### 修改配置

修改配置文件后，程序会在 **60 秒内自动应用新配置**，无需手动重启服务。

```bash
# TOML 版本
vim /etc/nat.toml

# 传统版本
vim /etc/nat.conf
```

### 查看日志

```bash
# 实时查看日志
journalctl -fu nat

# 查看详细日志
journalctl -exfu nat

# 查看最近 100 行日志
journalctl -u nat -n 100
```

### 查看 nftables 规则

```bash
# 查看所有规则
nft list ruleset

# 仅查看 NAT 表
nft list table ip self-nat
nft list table ip6 self-nat6
```

## 🔧 高级配置

### 自定义源 IP（多网卡场景）

默认使用 masquerade 自动处理 SNAT。如需指定源 IP：

```bash
# 设置自定义源 IP
echo "nat_local_ip=10.10.10.10" > /opt/nat/env

# 重启服务
systemctl restart nat
```

## 🐋 Docker 兼容性

本工具已与 Docker 完全兼容。程序会自动调整 nftables 规则以适配 Docker 网络。

> **说明**：Docker v28 将 filter 表 forward 链默认策略改为 DROP，本工具会自动将其重置为 ACCEPT 以确保 NAT 规则正常工作。

## 📌 注意事项

### REDIRECT 类型限制

`REDIRECT` 类型工作在 PREROUTING 链，仅对外部流量有效：

- ✅ **有效**：外部机器访问重定向端口 → 成功重定向
- ❌ **无效**：本机进程访问重定向端口 → 不会重定向

**原因**：本机流量直接进入 OUTPUT 链，不经过 PREROUTING 链。

**示例**：

```bash
# 配置：REDIRECT,8000,3128
curl http://remote-server:8000  # ✅ 成功重定向到 3128
curl http://localhost:8000      # ❌ 不会重定向，直接访问 8000
```

### TLS/Trojan 转发

转发 TLS/Trojan 等加密协议时，常见问题是证书配置错误。

**解决方案**：

1. **简单**：客户端禁用证书验证
2. **推荐**：正确配置证书和域名，确保证书域名与中转机匹配

## 📄 许可证

本项目采用 [MIT License](LICENSE) 开源协议。

## 🔗 相关链接

- **项目地址**：https://github.com/arloor/nftables-nat-rust
- **问题反馈**：https://github.com/arloor/nftables-nat-rust/issues
- **前代项目**：[arloor/iptablesUtils](https://github.com/arloor/iptablesUtils)（不兼容）

---

**注意**：与旧版 iptablesUtils 不兼容，切换时请先卸载旧版或重装系统。
