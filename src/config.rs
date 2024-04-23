#![deny(warnings)]
use crate::ip;
use log::info;
use std::env;
use std::fs;
use std::process::exit;

#[derive(Debug)]
pub enum Protocol {
    All,
    Tcp,
    Udp,
}

impl Protocol {
    fn tcp_prefix(&self) -> String {
        match &self {
            Protocol::All => "".to_string(),
            Protocol::Tcp => "".to_string(),
            Protocol::Udp => "#".to_string(),
        }
    }
    fn udp_prefix(&self) -> String {
        match &self {
            Protocol::All => "".to_string(),
            Protocol::Tcp => "#".to_string(),
            Protocol::Udp => "".to_string(),
        }
    }
}

impl From<Protocol> for String {
    fn from(protocol: Protocol) -> Self {
        match protocol {
            Protocol::Udp => "udp".into(),
            Protocol::Tcp => "tcp".into(),
            Protocol::All => "all".into(),
        }
    }
}

impl From<String> for Protocol {
    fn from(protocol: String) -> Self {
        match protocol {
            protocol if protocol == "udp" => Protocol::Udp,
            protocol if protocol == "UDP" => Protocol::Udp,
            protocol if protocol == "tcp" => Protocol::Tcp,
            protocol if protocol == "TCP" => Protocol::Tcp,
            _ => Protocol::All,
        }
    }
}

#[derive(Debug)]
pub enum NatCell {
    Single {
        src_port: i32,
        dst_port: i32,
        dst_domain: String,
        protocol: Protocol,
    },
    Range {
        port_start: i32,
        port_end: i32,
        dst_domain: String,
        protocol: Protocol,
    },
    Comment {
        content: String,
    },
}

impl NatCell {
    pub fn build(&self) -> String {
        let dst_domain = match &self {
            NatCell::Single { dst_domain, .. } => dst_domain,
            NatCell::Range { dst_domain, .. } => dst_domain,
            NatCell::Comment { content } => return content.clone(),
        };
        let dst_ip = match ip::remote_ip(dst_domain) {
            Ok(s) => s,
            Err(_) => return "".to_string(),
        };
        // 从环境变量读取本机ip或自动探测
        let local_ip = env::var("nat_local_ip").unwrap_or(match ip::local_ip() {
            Ok(s) => s,
            Err(_) => return "".to_string(),
        });

        match &self {
            NatCell::Range {
                port_start,
                port_end,
                dst_domain: _,
                protocol,
            } => {
                format!("# {cell:?}\n\
                    {tcpPrefix}add rule ip nat PREROUTING tcp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd}\n\
                    {udpPrefix}add rule ip nat PREROUTING udp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd}\n\
                    {tcpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} tcp dport {portStart}-{portEnd} counter snat to {localIP}\n\
                    {udpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} udp dport {portStart}-{portEnd} counter snat to {localIP}\n\n\
                    ", cell = self, portStart = port_start, portEnd = port_end, dstIp = dst_ip, localIP = local_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix())
            }
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
            } => {
                if dst_domain == "localhost" || dst_domain == "127.0.0.1" {
                    // 重定向到本机
                    format!("# {cell:?}\n\
                        {tcpPrefix}add rule ip nat PREROUTING tcp dport {localPort} redirect to :{remotePort}\n\
                        {udpPrefix}add rule ip nat PREROUTING udp dport {localPort} redirect to :{remotePort}\n\n\
                        ", cell = self, localPort = src_port, remotePort = dst_port, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix())
                } else {
                    // 转发到其他机器
                    format!("# {cell:?}\n\
                        {tcpPrefix}add rule ip nat PREROUTING tcp dport {localPort} counter dnat to {dstIp}:{dstPort}\n\
                        {udpPrefix}add rule ip nat PREROUTING udp dport {localPort} counter dnat to {dstIp}:{dstPort}\n\
                        {tcpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} tcp dport {dstPort} counter snat to {localIP}\n\
                        {udpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} udp dport {dstPort} counter snat to {localIP}\n\n\
                        ", cell = self, localPort = src_port, dstPort = dst_port, dstIp = dst_ip, localIP = local_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix())
                }
            }
            NatCell::Comment { .. } => "".to_string(),
        }
    }
}

pub fn example(conf: &String) {
    info!("请在 {} 编写转发规则，内容类似：", &conf);
    info!(
        "{}",
        "SINGLE,10000,443,baidu.com\n\
                    RANGE,1000,2000,baidu.com"
    )
}

pub fn read_config(conf: String) -> Vec<NatCell> {
    let mut nat_cells = vec![];
    let mut contents = match fs::read_to_string(&conf) {
        Ok(s) => s,
        Err(_e) => {
            example(&conf);
            exit(1);
        }
    };
    contents = contents.replace("\r\n", "\n");

    let strs = contents.split('\n');
    for str in strs {
        if str.trim().starts_with('#') {
            nat_cells.push(NatCell::Comment {
                content: str.trim().to_string()+"\n",
            });
            continue;
        }
        let cells = str.trim().split(',').collect::<Vec<&str>>();
        if cells.len() == 4 || cells.len() == 5 {
            let mut protocal: Protocol = Protocol::All;
            if cells.len() == 5 {
                protocal = cells[4].trim().to_string().into();
            }
            if cells[0].trim() == "RANGE" {
                nat_cells.push(NatCell::Range {
                    port_start: cells[1].trim().parse::<i32>().unwrap(),
                    port_end: cells[2].trim().parse::<i32>().unwrap(),
                    dst_domain: String::from(cells[3].trim()),
                    protocol: protocal,
                });
            } else if cells[0].trim() == "SINGLE" {
                nat_cells.push(NatCell::Single {
                    src_port: cells[1].trim().parse::<i32>().unwrap(),
                    dst_port: cells[2].trim().parse::<i32>().unwrap(),
                    dst_domain: String::from(cells[3].trim()),
                    protocol: protocal,
                });
            } else {
                info!("#! {} is not valid", str)
            }
        } else if !str.trim().is_empty() {
            info!("#! {} is not valid", str)
        }
    }
    nat_cells
}
