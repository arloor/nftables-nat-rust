-----------------------------------------------------------------

## nftables nat规则生成工具

用途：便捷地设置nat流量转发

> 适用于centos8、redhat8、fedora31和支持nftables的debian系linux发行版如debian10

## 电报讨论组

电报讨论组 https://t.me/popstary

## 准备工作

1. 关闭firewalld
2. 关闭selinux
3. 开启内核端口转发
4. 安装nftables（一般情况下，centos8默认包含nftables）

以下一键完成：

```shell
# 关闭firewalld
service firewalld stop
systemctl disable firewalld
# 关闭selinux
setenforce 0
sed -i 's/SELINUX=enforcing/SELINUX=disabled/' /etc/selinux/config  
# 修改内存参数，开启端口转发
echo 1 > /proc/sys/net/ipv4/ip_forward
sed -i '/^net.ipv4.ip_forward=0/'d /etc/sysctl.conf
sed -n '/^net.ipv4.ip_forward=1/'p /etc/sysctl.conf | grep -q "net.ipv4.ip_forward=1"
if [ $? -ne 0 ]; then
    echo -e "net.ipv4.ip_forward=1" >> /etc/sysctl.conf && sysctl -p
fi
# 确保nftables已安装
yum install -y  nftables
```

**debian系说明** 请自行使用apt安装nftables，并禁用iptables

## 使用说明

```
# 必须是root用户
# sudo su

# 下载可执行文件
wget -O /usr/local/bin/nat http://cdn.arloor.com/tool/dnat
chmod +x /usr/local/bin/nat

# 创建systemd服务
cat > /lib/systemd/system/nat.service <<EOF
[Unit]
Description=dnat-service
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/nat /etc/nat.conf
LimitNOFILE=100000
Restart=always
RestartSec=60

[Install]
WantedBy=multi-user.target
EOF

# 设置开机启动，并启动该服务
systemctl daemon-reload
systemctl enable nat

# 生成配置文件，配置文件可按需求修改（请看下文）
cat > /etc/nat.conf <<EOF
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
EOF

systemctl start nat
```

**自定义转发规则**

`/etc/nat.conf`如下：

```$xslt
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
```

- 每行代表一个规则；行内以英文逗号分隔为4段内容
- SINGLE：单端口转发：本机49999端口转发到baidu.com:59999
- RANGE：范围端口转发：本机50000-50010转发到baidu.com:50000-50010
- 请确保配置文件符合格式要求，否则程序可能会出现不可预期的错误，包括但不限于你和你的服务器炸掉（认真

高级用法：

1. **转发到本地**：行尾域名处填写localhost即可，例如`SINGLE,2222,22,localhost`，表示本机的2222端口重定向到本机的22端口。
2. **仅转发tcp/udp流量**：行尾增加tcp/udp即可，例如`SINGLE,10000,443,baidu.com,tcp`表示仅转发tcp流量，`SINGLE,10000,443,baidu.com,udp`仅转发udp流量

如需修改转发规则，请`vim /etc/nat.conf`以设定你想要的转发规则。修改完毕后，无需重新启动vps或服务，程序将会自动在最多一分钟内更新nat转发规则（PS：受dns缓存影响，可能会超过一分钟）

## 优势

1. 实现动态nat：自动探测配置文件和目标域名IP的变化，除变更配置外无需任何手工介入
2. 支持IP和域名
3. 以配置文件保存转发规则，可备份或迁移到其他机器
4. 自动探测本机ip
5. 开机自启动
6. 支持端口段

## 一些需要注意的东西

1. 不支持多网卡
2. 本工具在centos8、redhat8、fedora31上有效，其他发行版未作测试
3. 与前作[arloor/iptablesUtils](https://github.com/arloor/iptablesUtils)不兼容，在两个工具之间切换时，请重装系统以确保系统纯净！

## 如何停止以及卸载

```shell
## 停止定时监听域名解析地任务
service nat stop
## 清空nat规则
nft add table ip nat
nft delete table ip nat
## 禁止开机启动
systemctl disable nat
```

## 致谢

1. [解决会清空防火墙的问题](https://github.com/arloor/nftables-nat-rust/pull/6)
2. [ubuntu18.04适配](https://github.com/arloor/nftables-nat-rust/issues/1)

## 常问问题

### 关于trojan转发

总是有人说，不能转发trojan，这么说的人大部分是证书配置不对。最简单的解决方案是：客户端选择不验证证书。复杂一点是自己把证书和中转机的域名搭配好。

小白记住一句话就好：客户端不验证证书。

### 如何查看最终的nftables规则

```shell
nft list ruleset
```

## 赏个鸡腿吧

<img src="/wechat_shoukuan.jpg" alt="" width="400px" style="max-width: 100%;">
