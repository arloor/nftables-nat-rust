-----------------------------------------------------------------

## centos8 nftables nat规则生成工具

> 仅适用于centos8、redhat8、fedora31

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


## 使用说明

```
# 必须是root用户
# sudo su

# 下载可执行文件
wget -O /usr/local/bin/nat http://cdn.arloor.com/tool/dnat
chmod +x /usr/local/bin/nat

# 生成配置文件，配置文件可按需求修改（请看下文）
cat > /etc/nat.conf <<EOF
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
EOF

# 创建systemd服务
cat > /lib/systemd/system/nat.service <<EOF
[Unit]
Description=动态设置nat规则
After=network-online.target
Wants=network-online.target

[Service]
WorkingDirectory=/opt/socks5
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
systemctl start nat
```

**配置文件内容说明**

`/etc/nat.conf`如下：

```$xslt
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
```

- 每行代表一个规则；每行以英文逗号分隔；逗号前后不能有空格
- SINGLE：单端口转发：本机49999端口转发到baidu.com:59999
- RANGE：范围端口转发：本机50000-50010转发到baidu.com:50000-50010

请`vim /etc/nat.conf`以设定你想要的转发规则。修改完毕后，无需重新启动vps或服务，将会自动在最多一分钟内更新nat转发规则（PS：受dns缓存影响，可能会超过一分钟）


## 优势

1. 实现动态nat：自动探测配置文件和目标域名IP的变化
2. 支持IP和域名
3. 以配置文件保存转发规则，可备份或迁移到其他机器
4. 自动探测本机ip

## 一些需要注意的东西

1. 本工具会清空所有防火墙规则（当然，防火墙没那么重要～
2. 本机多个网卡的情况未作测试（大概率会有问题）
3. 本工具在centos8、redhat8、fedora31上有效，其他发行版未作测试
4. 与前作[arloor/iptablesUtils](https://github.com/arloor/iptablesUtils)不兼容，在两个工具之间切换时，请重装系统以确保系统纯净！
