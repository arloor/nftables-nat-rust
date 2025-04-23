## 基于nftables的端口转发管理工具

用途：便捷地设置nat流量转发

> 适用于centos8及以后的redhat系发行版和支持nftables的debian系linux发行版如debian10

## 优势

1. 实现动态nat：自动探测配置文件和目标域名IP的变化，除变更配置外无需任何手工介入
2. 支持IP和域名
3. 支持单独转发tcp或udp
4. 支持转发到本机其他端口（nat重定向）【2023.1.17更新】
5. 以配置文件保存转发规则，可备份或迁移到其他机器
6. 自动探测本机ip
7. 支持自定义本机ip【2023.1.17更新】
8. 开机自启动
9. 支持端口段
10. 兼容Docker
11. 轻量，只依赖rust标准库和日志库

## 准备工作

1. 关闭firewalld
2. 关闭selinux
3. 开启内核端口转发
4. 安装nftables（一般情况下，centos8默认包含nftables）

以下是**Centos8/9**上一键完成的脚本：

```shell
# 关闭firewalld
systemctl disable --now firewalld
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

**Debian系**请自行使用apt安装nftables，并禁用iptables

## 使用说明

```shell
# 必须是root用户
# sudo su
# 下载可执行文件
curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v1.0.0/dnat -o /tmp/nat
install /tmp/nat /usr/local/bin/nat

# 创建systemd服务
cat > /lib/systemd/system/nat.service <<EOF
[Unit]
Description=dnat-service
After=network-online.target
Wants=network-online.target

[Service]
WorkingDirectory=/opt/nat
EnvironmentFile=/opt/nat/env
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

mkdir /opt/nat
touch /opt/nat/env

# 生成配置文件，配置文件可按需求修改（请看下文）
cat > /etc/nat.conf <<EOF
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
EOF

systemctl restart nat
```

**配置文件说明**

`/etc/nat.conf` 如下：

```$xslt
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
```

- 每行代表一个规则；行内以英文逗号分隔为4段内容
- SINGLE：单端口转发：本机49999端口转发到baidu.com:59999
- RANGE：范围端口转发：本机50000-50010转发到baidu.com:50000-50010
- 请确保配置文件符合格式要求，否则程序可能会出现不可预期的错误，包括但不限于你和你的服务器炸掉（认真
- 以 `#` 开始的行会被当成注释

高级用法：

1. **转发到本地**：行尾域名处填写localhost即可，例如`SINGLE,2222,22,localhost`，表示本机的2222端口重定向到本机的22端口。
2. **仅转发tcp/udp流量**：行尾增加tcp/udp即可，例如`SINGLE,10000,443,baidu.com,tcp`表示仅转发tcp流量，`SINGLE,10000,443,baidu.com,udp`仅转发udp流量

如需修改转发规则，请`vim /etc/nat.conf`以设定你想要的转发规则。修改完毕后，无需重新启动vps或服务，程序将会自动在最多一分钟内更新nat转发规则（PS：受dns缓存影响，可能会超过一分钟）

## 更新新版

本程序由github actions自动发布新v1.0.0版本，可以通过下面的命令更新：

```bash
curl -sSLf https://us.arloor.dev/https://github.com/arloor/nftables-nat-rust/releases/download/v1.0.0/dnat -o /tmp/nat
install /tmp/nat /usr/local/bin/nat
systemctl restart nat
```

## 其他

1. 本工具在centos8、redhat8、fedora31上有效，其他发行版未作测试
2. 与前作[arloor/iptablesUtils](https://github.com/arloor/iptablesUtils)不兼容，在两个工具之间切换时，请先卸载原来的工具或重装系统

## 如何停止以及卸载

```shell
## 停止定时监听域名解析地任务
systemctl disable --now nat
## 清空nat规则
nft add table ip self-nat
nft delete table ip self-nat
```

## webui

感谢 @C018 贡献的[webui](webui/README.md)

## 致谢

1. [通过自定义nftables表名来避免与docker等服务冲突](https://github.com/arloor/nftables-nat-rust/pull/34)
2. [解决会清空防火墙的问题](https://github.com/arloor/nftables-nat-rust/pull/6)
3. [ubuntu18.04适配](https://github.com/arloor/nftables-nat-rust/issues/1)

## 常见问题

### docker兼容性

最新版本已经与docker兼容，欢迎试用和反馈。

> 更多说明：Docker v28 将filter表forward链的默认策略设置为了drop（参见[Docker Engine v28: Hardening Container Networking by Default](https://www.docker.com/blog/docker-engine-28-hardening-container-networking-by-default/)），这会导致我们的自定义nat规则无法通过forward链。为了解决此问题，此程序会自动将filter表forward链的默认策略重置为accept。

### 多网卡机器指定ip

```bash
echo "nat_local_ip=10.10.10.10" > /opt/nat/env #自定义本机ip，用于多网卡的机器
systemctl restart nat
```

### 关于trojan转发

总是有人说，不能转发trojan，这么说的人大部分是证书配置不对。最简单的解决方案是：客户端选择不验证证书。复杂一点是自己把证书和中转机的域名搭配好。

小白记住一句话就好：客户端不验证证书。

### 用于多网卡的机器时，如何指定用于转发的本机ip

可以执行以下脚本来自定义本机ip，该示例是将本机ip定义为`10.10.10.10`

```shell
echo "nat_local_ip=10.10.10.10" > /opt/nat/env
```

### 如何查看最终的nftables规则

```shell
nft list ruleset
```

### 查看日志

执行

```shell
journalctl -exfu nat
```

