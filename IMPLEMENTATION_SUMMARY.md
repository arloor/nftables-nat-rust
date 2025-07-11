# nftables-nat-rust IPv6支持总结

## 完成的功能

### ✅ 核心IPv6支持
1. **IP版本枚举** - 添加了`IpVersion`枚举支持V4/V6/Both
2. **配置解析扩展** - 支持在TOML和传统配置中指定IP版本
3. **DNS解析增强** - 分别支持IPv4和IPv6地址解析
4. **nftables规则生成** - 生成正确的IPv4和IPv6规则

### ✅ 内核参数配置
- 自动设置IPv4转发: `/proc/sys/net/ipv4/ip_forward = 1`
- 自动设置IPv6转发: `/proc/sys/net/ipv6/conf/all/forwarding = 1`
- IPv6配置失败时优雅降级（不中断程序）

### ✅ nftables脚本生成
- 创建IPv4表: `add table ip self-nat`
- 创建IPv6表: `add table ip6 self-nat`
- IPv6地址正确使用方括号格式: `[2400:da00::6666]:443`
- 支持IPv6本地重定向: `ip6 ... redirect to :port`
- 支持IPv6 DNAT和SNAT规则

### ✅ 配置文件增强
- TOML格式新增`ip_version`字段
- 传统格式支持第6个字段指定IP版本
- 向后兼容：默认为IPv4
- 智能IP版本检测

### ✅ 链检查和准备
- 检查IPv4和IPv6 FORWARD链策略
- 自动修复DROP策略为ACCEPT
- 支持Docker/iptables-nft环境

## 测试验证

### ✅ 配置解析测试
- 成功解析IPv4、IPv6和双栈配置
- 正确处理注释和不同协议
- DNS解析正常工作

### ✅ 脚本生成测试
- IPv4规则生成正确
- IPv6规则使用正确语法
- 双栈配置智能选择IP版本

## 使用示例

### TOML配置
```toml
[[rules]]
type = "single"
sport = 8080
dport = 80
domain = "::1"
protocol = "tcp"
ip_version = "ipv6"
comment = "IPv6 localhost redirect"
```

### 传统配置
```
SINGLE,8080,80,::1,tcp,ipv6
```

### 生成的IPv6规则
```nft
add table ip6 self-nat
add chain self-nat PREROUTING { type nat hook prerouting priority -110 ; }
add rule ip6 self-nat PREROUTING tcp dport 8080 redirect to :80
```

## 向后兼容性

✅ 现有配置无需修改  
✅ 默认行为保持IPv4  
✅ 命令行参数不变  
✅ API接口保持兼容  

## 系统要求

- Linux内核支持IPv6
- nftables工具支持IPv6 NAT
- 系统启用IPv6网络栈

## 结论

成功为nftables-nat-rust添加了完整的IPv6 NAT转发支持，包括：
- 配置解析和验证
- DNS解析和IP检测
- nftables规则生成
- 内核参数设置
- 向后兼容性保证

该实现提供了灵活的IPv4/IPv6双栈NAT转发能力，满足现代网络环境的需求。
