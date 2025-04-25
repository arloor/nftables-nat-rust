#![deny(warnings)]
use crate::ip;
use log::info;
use std::env;
use std::fmt::Display;
use std::fs;
use std::io;

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

impl Display for NatCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
            } => write!(f, "SINGLE,{src_port},{dst_port},{dst_domain},{protocol:?}"),
            NatCell::Range {
                port_start,
                port_end,
                dst_domain,
                protocol,
            } => write!(f, "RANGE,{port_start},{port_end},{dst_domain},{protocol:?}"),
            NatCell::Comment { content } => write!(f, "{content}"),
        }
    }
}

impl NatCell {
    pub fn build(&self) -> Result<String, io::Error> {
        let dst_domain = match &self {
            NatCell::Single { dst_domain, .. } => dst_domain,
            NatCell::Range { dst_domain, .. } => dst_domain,
            NatCell::Comment { content } => return Ok(content.clone()),
        };
        let dst_ip = ip::remote_ip(dst_domain)?;
        // 从环境变量读取本机ip或自动探测
        let local_ip = env::var("nat_local_ip").unwrap_or(ip::local_ip()?);

        match &self {
            NatCell::Range {
                port_start,
                port_end,
                dst_domain: _,
                protocol,
            } => {
                let res=format!("{tcpPrefix}add rule ip self-nat PREROUTING tcp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd} comment \"{cell}\"\n\
                    {udpPrefix}add rule ip self-nat PREROUTING udp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd} comment \"{cell}\"\n\
                    {tcpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} tcp dport {portStart}-{portEnd} counter snat to {localIP} comment \"{cell}\"\n\
                    {udpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} udp dport {portStart}-{portEnd} counter snat to {localIP} comment \"{cell}\"\n\n\
                    ", cell = self, portStart = port_start, portEnd = port_end, dstIp = dst_ip, localIP = local_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
                Ok(res)
            }
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
            } => {
                match dst_domain.as_str() {
                    "localhost" | "127.0.0.1" => {
                        // 重定向到本机
                        let res = format!("{tcpPrefix}add rule ip self-nat PREROUTING tcp dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\
                            {udpPrefix}add rule ip self-nat PREROUTING udp dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\n\
                            ", cell = self, localPort = src_port, remotePort = dst_port, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
                        Ok(res)
                    },
                    _ => {
                        // 转发到其他机器
                        let res = format!("{tcpPrefix}add rule ip self-nat PREROUTING tcp dport {localPort} counter dnat to {dstIp}:{dstPort}  comment \"{cell}\"\n\
                            {udpPrefix}add rule ip self-nat PREROUTING udp dport {localPort} counter dnat to {dstIp}:{dstPort}  comment \"{cell}\"\n\
                            {tcpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} tcp dport {dstPort} counter snat to {localIP} comment \"{cell}\"\n\
                            {udpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} udp dport {dstPort} counter snat to {localIP} comment \"{cell}\"\n\n\
                            ", cell = self, localPort = src_port, dstPort = dst_port, dstIp = dst_ip, localIP = local_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
                        Ok(res)
                    }
                }
            }
            NatCell::Comment { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Comment cell cannot be built",
            )),
        }
    }

    /// 解析一行配置，返回NatCell或错误
    pub fn parse(line: &str) -> Result<Option<NatCell>, io::Error> {
        let line = line.trim();

        // 处理注释
        if line.starts_with('#') {
            return Ok(Some(NatCell::Comment {
                content: line.to_string() + "\n",
            }));
        }

        // 忽略空行
        if line.is_empty() {
            return Ok(None);
        }

        let cells: Vec<&str> = line.split(',').collect();

        // 验证字段数量
        if cells.len() != 4 && cells.len() != 5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("无效的配置行: {}, 字段数量不正确", line),
            ));
        }

        // 解析协议
        let protocol = if cells.len() == 5 {
            cells[4].trim().to_string().into()
        } else {
            Protocol::All
        };

        // 解析类型并创建NatCell
        match cells[0].trim() {
            "RANGE" => {
                let port_start = cells[1].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("无法解析起始端口: {}", e),
                    )
                })?;

                let port_end = cells[2].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("无法解析结束端口: {}", e),
                    )
                })?;

                Ok(Some(NatCell::Range {
                    port_start,
                    port_end,
                    dst_domain: cells[3].trim().to_string(),
                    protocol,
                }))
            }
            "SINGLE" => {
                let src_port = cells[1].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("无法解析源端口: {}", e))
                })?;

                let dst_port = cells[2].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("无法解析目标端口: {}", e),
                    )
                })?;

                Ok(Some(NatCell::Single {
                    src_port,
                    dst_port,
                    dst_domain: cells[3].trim().to_string(),
                    protocol,
                }))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("无效的转发规则类型: {}", cells[0].trim()),
            )),
        }
    }
}

pub(crate) fn example(conf: &str) {
    info!("请在 {} 编写转发规则，内容类似：", &conf);
    info!(
        "{}",
        "SINGLE,10000,443,baidu.com\n\
                    RANGE,1000,2000,baidu.com"
    )
}

pub fn read_config(conf: &str) -> Result<Vec<NatCell>, io::Error> {
    let mut nat_cells = vec![];
    let mut contents = fs::read_to_string(conf)?;
    contents = contents.replace("\r\n", "\n");

    let strs = contents.split('\n');
    for line in strs {
        if let Some(nat_cell) = NatCell::parse(line)? {
            nat_cells.push(nat_cell);
        }
    }
    Ok(nat_cells)
}
