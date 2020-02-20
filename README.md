## centos8 nftables nat规则生成工具

## 使用方式：

```
wget -O nat http://cdn.arloor.com/nat
chmod +x nat
./nat nat.conf
```

其中`nat.conf`类似如下：

```$xslt
SINGLE,443,443,baidu.com
RANGE,1000,2000,baidu.com
```

- 每行代表一个规则
- SINGLE：单端口转发：本机443端口转发到baidu.com:443
- RANGE：范围端口转发：本机1000-2000转发到baidu.com:1000-2000

## 执行示例

```$xslt
nftables脚本如下：
#!/usr/sbin/nft -f

flush ruleset
add table ip nat
add chain nat PREROUTING { type nat hook prerouting priority -100 ; }
add chain nat POSTROUTING { type nat hook postrouting priority 100 ; }

#SINGLE { local_port: 10000, remote_port: 443, remote_domain: "baidu.com" }
add rule ip nat PREROUTING tcp dport 10000 counter dnat to 39.156.69.79:443
add rule ip nat PREROUTING udp dport 10000 counter dnat to 39.156.69.79:443
add rule ip nat POSTROUTING ip daddr 39.156.69.79 tcp dport 443 counter snat to 172.17.37.225
add rule ip nat POSTROUTING ip daddr 39.156.69.79 udp dport 443 counter snat to 172.17.37.225

#RANGE { port_start: 1000, port_end: 2000, remote_domain: "baidu.com" }
add rule ip nat PREROUTING tcp dport 1000-2000 counter dnat to 220.181.38.148:1000-2000
add rule ip nat PREROUTING udp dport 1000-2000 counter dnat to 220.181.38.148:1000-2000
add rule ip nat POSTROUTING ip daddr 220.181.38.148 tcp dport 1000-2000 counter snat to 172.17.37.225
add rule ip nat POSTROUTING ip daddr 220.181.38.148 udp dport 1000-2000 counter snat to 172.17.37.225


执行/usr/sbin/nft -f temp.nft
执行结果: exit code: 0
```