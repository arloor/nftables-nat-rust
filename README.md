-----------------------------------------------------------------

## centos8 nftables nat规则生成工具

> 仅适用于centos8、redhat8

## 准备工作

1. 关闭firewalld
2. 关闭selinux
3. 开启内核端口转发

以下一键完成：

```$xslt
service firewalld stop
systemctl disable firewalld
setenforce 0
sed -i 's/SELINUX=enforcing/SELINUX=disabled/' /etc/selinux/config  
sed -n '/^net.ipv4.ip_forward=1/'p /etc/sysctl.conf | grep -q "net.ipv4.ip_forward=1"
if [ $? -ne 0 ]; then
    echo -e "net.ipv4.ip_forward=1" >> /etc/sysctl.conf && sysctl -p
fi
```


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

## 输出示例

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

## 注意

1. 重启会转发规则会失效，此时重新执行`./nat nat.conf`即可
2. 当本机ip或目标主机ip变化时，需要手动执行`./nat nat.conf`
3. 本机多个网卡的情况未作测试
4. 本工具在centos8上有效，其他发行版未作测试
