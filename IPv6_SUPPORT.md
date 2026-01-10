# IPv6 NAT 转发功能

本项目已成功扩展支持 IPv6 NAT 转发功能。以下是 IPv6 支持的详细说明：

## 新增功能

### 1. IPv6 协议支持

- 支持 IPv4、IPv6 以及两者同时支持的 NAT 转发规则
- 自动检测目标地址类型（IPv4 或 IPv6）
- 智能 DNS 解析，优先 IPv4，失败时使用 IPv6

### 2. IP 版本配置选项

在配置文件中新增 `ip_version` 字段，支持以下值：

- `ipv4`: 仅支持 IPv4 转发
- `ipv6`: 仅支持 IPv6 转发
- `all`: 自动检测目标地址类型

### 3. 内核参数配置

程序会自动配置以下内核参数：

- IPv4 转发: `/proc/sys/net/ipv4/ip_forward = 1`
- IPv6 转发: `/proc/sys/net/ipv6/conf/all/forwarding = 1`

### 4. nftables 规则生成

- 自动创建 IPv4 和 IPv6 NAT 表
- 为 IPv6 地址生成正确的 nftables 规则语法
- 支持 IPv6 地址的方括号格式

## 配置示例

### TOML 配置文件格式

```toml
[[rules]]
type = "single"
sport = 10000
dport = 443
domain = "example.com"
protocol = "all"
ip_version = "ipv4"
comment = "IPv4 HTTPS转发"

[[rules]]
type = "single"
sport = 10001
dport = 443
domain = "ipv6.example.com"
protocol = "all"
ip_version = "ipv6"
comment = "IPv6 HTTPS转发"

[[rules]]
type = "range"
port_start = 2000
port_end = 3000
domain = "dual-stack.example.com"
protocol = "tcp"
ip_version = "all"
comment = "双栈支持"
```

### 传统配置文件格式

```
SINGLE,10000,443,example.com,all,ipv4
SINGLE,10001,443,ipv6.example.com,all,ipv6
RANGE,2000,3000,dual-stack.example.com,tcp,all
```

## 生成的 nftables 规则示例

### IPv4 规则

```nft
add table ip self-nat
add chain self-nat PREROUTING { type nat hook prerouting priority -110 ; }
add chain self-nat POSTROUTING { type nat hook postrouting priority 110 ; }
add rule ip self-nat PREROUTING tcp dport 10000 counter dnat to 192.168.1.100:443
add rule ip self-nat POSTROUTING ip daddr 192.168.1.100 tcp dport 443 counter masquerade
```

### IPv6 规则

```nft
add table ip6 self-nat
add chain self-nat PREROUTING { type nat hook prerouting priority -110 ; }
add chain self-nat POSTROUTING { type nat hook postrouting priority 110 ; }
add rule ip6 self-nat PREROUTING tcp dport 10001 counter dnat to [2001:db8::1]:443
add rule ip6 self-nat POSTROUTING ip6 daddr 2001:db8::1 tcp dport 443 counter masquerade
```

## 环境变量支持

- `nat_local_ip`: 指定 IPv4 SNAT 源地址
- `nat_local_ipv6`: 指定 IPv6 SNAT 源地址

## 向后兼容性

- 现有配置文件无需修改，默认使用 IPv4
- 保持原有的 API 和命令行参数不变
- 新功能为可选扩展

## 系统要求

- Linux 内核支持 IPv6
- nftables 工具版本支持 IPv6 NAT
- 系统已启用 IPv6 网络栈
