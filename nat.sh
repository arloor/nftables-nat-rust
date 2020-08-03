#!/usr/bin/env bash

# 说明
[ 0 -eq 1 ] && {
1. 使用定时任务运行转发脚本
   wget --no-check-certificate -O /opt/nft-nat.sh https://raw.githubusercontent.com/arloor/nftables-nat-rust/master/nat.sh
   chmod 755 /opt/nft-nat.sh
   (crontab -l ; echo "0 */2 * * * /opt/nft-nat.sh") | crontab -
2. 配置文件/etc/nat.conf格式：
SINGLE,49999,59999,baidu.com
RANGE,50000,50010,baidu.com
}

### dependencies
command -v nft > /dev/null 2>&1 || { echo "Please install nftables"; exit 1; }

###
[[ ! -f /etc/nat.conf ]] && echo Sorry, no File: /etc/nat.conf && exit 1

### 
cat /etc/sysctl.conf | grep -qwE "^#net.ipv4.ip_forward=1" && sed -i "s/^#net.ipv4.ip_forward=1/net.ipv4.ip_forward=1/" /etc/sysctl.conf
echo 1 > /proc/sys/net/ipv4/ip_forward
sysctl -p >/dev/null

###
nft add table ip nat
nft delete table ip nat
nft add table ip nat
nft add chain nat PREROUTING { type nat hook prerouting priority \ -100 \; }
nft add chain nat POSTROUTING { type nat hook postrouting priority 100 \; }

###
local_ip=$(ip address | grep -E "scope global" | head -n1 | cut -f6 -d" " | cut -f1 -d"/")

for ((i=1; i<=$(cat /etc/nat.conf | grep -c ""); i++)); do

    remote_ip=$(ping -w 1 -c 1 $(cat /etc/nat.conf | sed -n "${i}p" | cut -f4 -d,) | head -n 1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+" | head -n 1)
    
    if cat /etc/nat.conf | sed -n "${i}p" | grep -qE "^SINGLE"; then
        local_port=$(cat /etc/nat.conf | sed -n "${i}p" | cut -f2 -d,)
        remote_port=$(cat /etc/nat.conf | sed -n "${i}p" | cut -f3 -d,)
    elif cat /etc/nat.conf | sed -n "${i}p" | grep -qE "^RANGE"; then
        local_port="$(cat /etc/nat.conf | sed -n "${i}p" | cut -f2 -d,)-$(cat /etc/nat.conf | sed -n "${i}p" | cut -f3 -d,)"
        remote_port=$local_port
    else 
        echo Err config: /etc/nat.conf; exit 1
    fi

    nft add rule ip nat PREROUTING tcp dport $local_port counter dnat to $remote_ip:$remote_port
    nft add rule ip nat PREROUTING udp dport $local_port counter dnat to $remote_ip:$remote_port
    
    nft add rule ip nat POSTROUTING ip daddr $remote_ip tcp dport $remote_port counter snat to $local_ip
    nft add rule ip nat POSTROUTING ip daddr $remote_ip udp dport $remote_port counter snat to $local_ip
    
done

nft list ruleset
