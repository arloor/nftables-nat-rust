# IPv6 NAT 转发功能

本项目已成功扩展支持IPv6 NAT转发功能。以下是IPv6支持的详细说明：

## 新增功能

### 1. IPv6协议支持
- 支持IPv4、IPv6以及两者同时支持的NAT转发规则
- 自动检测目标地址类型（IPv4或IPv6）
- 智能DNS解析，优先IPv4，失败时使用IPv6

### 2. IP版本配置选项
在配置文件中新增 `ip_version` 字段，支持以下值：
- `ipv4`: 仅支持IPv4转发
- `ipv6`: 仅支持IPv6转发  
- `both`: 自动检测目标地址类型

### 3. 内核参数配置
程序会自动配置以下内核参数：
- IPv4转发: `/proc/sys/net/ipv4/ip_forward = 1`
- IPv6转发: `/proc/sys/net/ipv6/conf/all/forwarding = 1`

### 4. nftables规则生成
- 自动创建IPv4和IPv6 NAT表
- 为IPv6地址生成正确的nftables规则语法
- 支持IPv6地址的方括号格式

## 配置示例

### TOML配置文件格式
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
portStart = 2000
portEnd = 3000
domain = "dual-stack.example.com"
protocol = "tcp"
ip_version = "both"
comment = "双栈支持"
```

### 传统配置文件格式
```
SINGLE,10000,443,example.com,all,ipv4
SINGLE,10001,443,ipv6.example.com,all,ipv6
RANGE,2000,3000,dual-stack.example.com,tcp,both
```

## 生成的nftables规则示例

### IPv4规则
```nft
add table ip self-nat
add chain self-nat PREROUTING { type nat hook prerouting priority -110 ; }
add chain self-nat POSTROUTING { type nat hook postrouting priority 110 ; }
add rule ip self-nat PREROUTING tcp dport 10000 counter dnat to 192.168.1.100:443
add rule ip self-nat POSTROUTING ip daddr 192.168.1.100 tcp dport 443 counter masquerade
```

### IPv6规则
```nft
add table ip6 self-nat
add chain self-nat PREROUTING { type nat hook prerouting priority -110 ; }
add chain self-nat POSTROUTING { type nat hook postrouting priority 110 ; }
add rule ip6 self-nat PREROUTING tcp dport 10001 counter dnat to [2001:db8::1]:443
add rule ip6 self-nat POSTROUTING ip6 daddr 2001:db8::1 tcp dport 443 counter masquerade
```

## 环境变量支持

- `nat_local_ip`: 指定IPv4 SNAT源地址
- `nat_local_ipv6`: 指定IPv6 SNAT源地址

## 向后兼容性

- 现有配置文件无需修改，默认使用IPv4
- 保持原有的API和命令行参数不变
- 新功能为可选扩展

## 系统要求

- Linux内核支持IPv6
- nftables工具版本支持IPv6 NAT
- 系统已启用IPv6网络栈
