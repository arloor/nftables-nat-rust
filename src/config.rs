// SINGLE,8100,8100,arloor.com
// RANGE,1000,2000,arloor.com
use std::fs::{self, File};
use crate::ip;
use std::process::exit;

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    ALL,
    TCP,
    UDP,
}

impl Protocol {
    fn tcp_prefix(&self) -> String {
        match &self {
            Protocol::ALL => "".to_string(),
            Protocol::TCP => "".to_string(),
            Protocol::UDP => "#".to_string(),
        }
    }
    fn udp_prefix(&self) -> String {
        match &self {
            Protocol::ALL => "".to_string(),
            Protocol::TCP => "#".to_string(),
            Protocol::UDP => "".to_string(),
        }
    }
}

impl From<Protocol> for String {
    fn from(protocol: Protocol) -> Self {
        match protocol {
            Protocol::UDP => "udp".into(),
            Protocol::TCP => "tcp".into(),
            Protocol::ALL => "all".into(),
        }
    }
}

impl From<String> for Protocol {
    fn from(protocol: String) -> Self {
        match protocol {
            protocol if protocol == "udp" => Protocol::UDP,
            protocol if protocol == "UDP" => Protocol::UDP,
            protocol if protocol == "tcp" => Protocol::TCP,
            protocol if protocol == "TCP" => Protocol::TCP,
            _ => Protocol::ALL,
        }
    }
}

#[derive(Debug)]
pub enum NatCell {
    SINGLE {
        src_port: i32,
        dst_port: i32,
        dst_domain: String,
        protocol: Protocol,
    },
    RANGE {
        port_start: i32,
        port_end: i32,
        dst_domain: String,
        protocol: Protocol,
    },
}

impl NatCell {
    pub fn build(&self) -> String {
        let dst_domain = match &self {
            NatCell::SINGLE { dst_domain, .. } => dst_domain,
            NatCell::RANGE { dst_domain, .. } => dst_domain
        };
        let dst_ip = match ip::remote_ip(dst_domain) {
            Some(s) => s,
            None => return "".to_string(),
        };
        let local_ip = match ip::local_ip() {
            Some(s) => s,
            None => return "".to_string(),
        };

        match &self {
            NatCell::RANGE { port_start, port_end, dst_domain, protocol } =>
                {
                    format!("#{cell:?}\n\
                    {tcpPrefix}add rule ip nat PREROUTING tcp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd}\n\
                    {udpPrefix}add rule ip nat PREROUTING udp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd}\n\
                    {tcpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} tcp dport {portStart}-{portEnd} counter snat to {localIP}\n\
                    {udpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} udp dport {portStart}-{portEnd} counter snat to {localIP}\n\n\
                    ", cell = self, portStart = port_start, portEnd = port_end, dstIp = dst_ip, localIP = local_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix())
                }
            NatCell::SINGLE { src_port, dst_port, dst_domain, protocol } =>
                {
                    if dst_domain == "localhost" || dst_domain == "127.0.0.1" { // 重定向到本机
                        format!("#{cell:?}\n\
                        {tcpPrefix}add rule ip nat PREROUTING tcp dport {localPort} redirect to :{remotePort}\n\
                        {udpPrefix}add rule ip nat PREROUTING udp dport {localPort} redirect to :{remotePort}\n\n\
                        ", cell = self, localPort = src_port, remotePort = dst_port, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix())
                    } else { // 转发到其他机器
                        format!("#{cell:?}\n\
                        {tcpPrefix}add rule ip nat PREROUTING tcp dport {localPort} counter dnat to {dstIp}:{dstPort}\n\
                        {udpPrefix}add rule ip nat PREROUTING udp dport {localPort} counter dnat to {dstIp}:{dstPort}\n\
                        {tcpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} tcp dport {dstPort} counter snat to {localIP}\n\
                        {udpPrefix}add rule ip nat POSTROUTING ip daddr {dstIp} udp dport {dstPort} counter snat to {localIP}\n\n\
                        ", cell = self, localPort = src_port, dstPort = dst_port, dstIp = dst_ip, localIP = local_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix())
                    }
                }
        }
    }

    pub fn get_target_ip(&self) -> (String, String) {
        match &self {
            NatCell::RANGE { port_start, port_end, dst_domain: remote_domain, protocol } =>
                (remote_domain.clone(), match ip::remote_ip(remote_domain) {
                    Some(s) => s,
                    None => "".to_string()
                })
            ,
            NatCell::SINGLE { src_port: local_port, dst_port: remote_port, dst_domain: remote_domain, protocol } =>
                (remote_domain.clone(), match ip::remote_ip(remote_domain) {
                    Some(s) => s,
                    None => "".to_string()
                })
        }
    }
}


pub fn example(conf: &String) {
    println!("请在 {} 编写转发规则，内容类似：", &conf);
    println!("{}", "SINGLE,10000,443,baidu.com\n\
                    RANGE,1000,2000,baidu.com")
}

pub fn read_config(conf: String) -> Vec<NatCell> {
    let mut nat_cells = vec![];
    let mut contents = match fs::read_to_string(&conf) {
        Ok(s) => s,
        Err(e) => {
            example(&conf);
            exit(1);
        }
    };
    contents = contents.replace("\r\n", "\n");

    let strs = contents.split("\n");
    for str in strs {
        let cells = str.trim().split(",").collect::<Vec<&str>>();
        if cells.len() == 4 || cells.len() == 5 {
            let mut protocal: Protocol = Protocol::ALL;
            if cells.len() == 5 {
                protocal = cells[4].trim().to_string().into();
            }
            if cells[0].trim() == "RANGE" {
                nat_cells.push(NatCell::RANGE {
                    port_start: cells[1].trim().parse::<i32>().unwrap(),
                    port_end: cells[2].trim().parse::<i32>().unwrap(),
                    dst_domain: String::from(cells[3].trim()),
                    protocol: protocal,
                });
            }
            if cells[0].trim() == "SINGLE" {
                nat_cells.push(NatCell::SINGLE {
                    src_port: cells[1].trim().parse::<i32>().unwrap(),
                    dst_port: cells[2].trim().parse::<i32>().unwrap(),
                    dst_domain: String::from(cells[3].trim()),
                    protocol: protocal,
                });
            }
        } else if str.trim().len() != 0 {
            println!("#! {} is not valid", str)
        }
    }
    nat_cells
}