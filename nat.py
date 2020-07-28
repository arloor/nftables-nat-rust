import socket
import sys
import time
import subprocess

# 配置文件如下
'''
SINGLE,80,80,baidu.com
SINGLE,99,99,baidu.com,tcp
SINGLE,100,100,baidu.com,udp
RANGE,100,200,baidu.com
'''

nft_tmp_file = "nat-diy.nft"

nf_prefix = '''#!/usr/sbin/nft -f\n\
\n\
add table ip nat\n\
delete table ip nat\n\
add table ip nat\n\
add chain nat PREROUTING { type nat hook prerouting priority -100 ; }\n\
add chain nat POSTROUTING { type nat hook postrouting priority 100 ; }\n\n'''

nf_format = {
    "RANGE": {
        "tcp": "add rule ip nat PREROUTING tcp dport {port1}-{port2} counter dnat to {remoteIP}:{port1}-{port2}\nadd rule ip nat POSTROUTING ip daddr {remoteIP} tcp dport {port1}-{port2} counter snat to {localIP}\n",
        "udp": "add rule ip nat PREROUTING udp dport {port1}-{port2} counter dnat to {remoteIP}:{port1}-{port2}\nadd rule ip nat POSTROUTING ip daddr {remoteIP} udp dport {port1}-{port2} counter snat to {localIP}\n"
    },
    "SINGLE": {
        "tcp": "add rule ip nat PREROUTING tcp dport {port1} counter dnat to {remoteIP}:{port2}\nadd rule ip nat POSTROUTING ip daddr {remoteIP} tcp dport {port2} counter snat to {localIP}\n",
        "udp": "add rule ip nat PREROUTING udp dport {port1} counter dnat to {remoteIP}:{port2}\nadd rule ip nat POSTROUTING ip daddr {remoteIP} udp dport {port2} counter snat to {localIP}\n"
    }
}

def query(domain):
    addr = None
    with socket.socket(type=socket.SOCK_DGRAM) as s:
        try:
            s.connect((domain, 80))
            addr = s.getpeername()
            return addr[0]
        except socket.gaierror as err:
            print("cannot find %s, %s" % (domain, err))
    return addr

def local():
    local_addr = None
    with socket.socket(type=socket.SOCK_DGRAM) as s:
        try:
            s.connect(("8.8.8.8", 80))
            local_addr = s.getsockname()
            return local_addr[0]
        except socket.gaierror as err:
            pass
    return local_addr

def read_conf():
    args = sys.argv
    cells = []
    if len(args) != 2:
        print("使用方式: nat nat.conf")
        exit(-1)
    else:
        conf_file = args[1]
        try:
            with open(conf_file, mode="r") as conf:
                for line in conf.readlines():
                    cell = line.strip().split(",")
                    cells.append(cell)
        except FileNotFoundError as err:
            print("配置文件找不到, err: %s" % err)
            exit(-1)
    return cells


def generate_nftables(conf_cells):
    localIP = local()
    result = nf_prefix
    for cell in conf_cells:
        if len(cell) < 4:
            pass
        else:
            domain = cell[3]
            remoteIP = query(domain)
            result += "# %s\n" % cell
            if remoteIP:
                port1 = cell[1]
                port2 = cell[2]

                format = nf_format[cell[0]]
                if len(cell) == 5:
                    if cell[4] == "tcp":
                        result += format["tcp"].format(port1=port1, port2=port2, localIP=localIP, remoteIP=remoteIP)
                    if cell[4] == "udp":
                        result += format["udp"].format(port1=port1, port2=port2, localIP=localIP, remoteIP=remoteIP)
                else:
                    result += format["tcp"].format(port1=port1, port2=port2, localIP=localIP, remoteIP=remoteIP)
                    result += format["udp"].format(port1=port1, port2=port2, localIP=localIP, remoteIP=remoteIP)
    return result


if __name__ == "__main__":
    old_nft_rule = ""
    while True:
        conf_cells = read_conf()
        nft_rule = generate_nftables(conf_cells)
        if nft_rule != old_nft_rule:
            # 写
            print("新的nat规则如下\n")
            print(nft_rule)
            with open(nft_tmp_file, mode="w") as tmp:
                tmp.write(nft_rule)
                tmp.flush()
                try:
                    p = subprocess.run('/usr/sbin/nft -f %s' % nft_tmp_file, check=True,shell=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=1)
                except subprocess.CalledProcessError as err:
                    print("更新nftables规则失败，err：%s\n" % err)
                else:
                    print("更新nftables成功，等待发现变更\n")
            old_nft_rule = nft_rule
        time.sleep(120)
