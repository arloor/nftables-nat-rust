#![deny(warnings)]
use crate::ip;
use log::info;
use serde::Deserialize;
use serde::Serialize;
use std::env;
use std::fmt::Display;
use std::fs;
use std::io;

#[derive(Debug, Clone)]
pub enum IpVersion {
    V4,
    V6,
    Both, // 优先IPv4，如果IPv4不可用则使用IPv6
}

impl From<String> for IpVersion {
    fn from(version: String) -> Self {
        match version.to_lowercase().as_str() {
            "ipv4" | "v4" | "4" => IpVersion::V4,
            "ipv6" | "v6" | "6" => IpVersion::V6,
            "both" | "all" => IpVersion::Both,
            _ => IpVersion::Both,
        }
    }
}

impl Display for IpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpVersion::V4 => write!(f, "IPv4"),
            IpVersion::V6 => write!(f, "IPv6"),
            IpVersion::Both => write!(f, "Both"),
        }
    }
}

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
            protocol if protocol == "Udp" => Protocol::Udp,
            protocol if protocol == "UDP" => Protocol::Udp,
            protocol if protocol == "tcp" => Protocol::Tcp,
            protocol if protocol == "Tcp" => Protocol::Tcp,
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
        ip_version: IpVersion,
    },
    Range {
        port_start: i32,
        port_end: i32,
        dst_domain: String,
        protocol: Protocol,
        ip_version: IpVersion,
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
                ip_version,
            } => write!(
                f,
                "SINGLE,{src_port},{dst_port},{dst_domain},{protocol:?},{ip_version}"
            ),
            NatCell::Range {
                port_start,
                port_end,
                dst_domain,
                protocol,
                ip_version,
            } => write!(
                f,
                "RANGE,{port_start},{port_end},{dst_domain},{protocol:?},{ip_version}"
            ),
            NatCell::Comment { content } => write!(f, "{content}"),
        }
    }
}

impl NatCell {
    pub fn build(&self) -> Result<String, io::Error> {
        let (dst_domain, ip_version) = match &self {
            NatCell::Single {
                dst_domain,
                ip_version,
                ..
            } => (dst_domain, ip_version),
            NatCell::Range {
                dst_domain,
                ip_version,
                ..
            } => (dst_domain, ip_version),
            NatCell::Comment { content } => return Ok(content.clone() + "\n"),
        };

        // 根据配置的IP版本解析目标IP
        let dst_ip = ip::remote_ip(dst_domain, ip_version)?;

        let mut result = String::new();

        // 检测实际IP类型并生成相应的规则
        let is_ipv6_target = dst_ip.contains(':');

        match ip_version {
            IpVersion::V4 => {
                if is_ipv6_target {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "IPv6 target address resolved but rule is configured for IPv4 only",
                    ));
                }
                result += &self.build_ipv4_rules(&dst_ip)?;
            }
            IpVersion::V6 => {
                if !is_ipv6_target {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "IPv4 target address resolved but rule is configured for IPv6 only",
                    ));
                }
                result += &self.build_ipv6_rules(&dst_ip)?;
            }
            IpVersion::Both => {
                if is_ipv6_target {
                    result += &self.build_ipv6_rules(&dst_ip)?;
                } else {
                    result += &self.build_ipv4_rules(&dst_ip)?;
                }
            }
        }

        Ok(result)
    }

    fn build_ipv4_rules(&self, dst_ip: &str) -> Result<String, io::Error> {
        // 从环境变量读取本机ip或自动探测
        let local_ip = env::var("nat_local_ip");
        let snat_to_part = match local_ip {
            Ok(ip) => "snat to ".to_owned() + &ip,
            Err(_) => "masquerade".to_owned(),
        };

        match &self {
            NatCell::Range {
                port_start,
                port_end,
                dst_domain: _,
                protocol,
                ip_version: _,
            } => {
                let res=format!("{tcpPrefix}add rule ip self-nat PREROUTING tcp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd} comment \"{cell}\"\n\
                    {udpPrefix}add rule ip self-nat PREROUTING udp dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd} comment \"{cell}\"\n\
                    {tcpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} tcp dport {portStart}-{portEnd} counter {snat_to_part} comment \"{cell}\"\n\
                    {udpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} udp dport {portStart}-{portEnd} counter {snat_to_part} comment \"{cell}\"\n\n\
                    ", cell = self, portStart = port_start, portEnd = port_end, dstIp = dst_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
                Ok(res)
            }
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
                ip_version: _,
            } => {
                match dst_domain.as_str() {
                    "localhost" | "127.0.0.1" => {
                        // 重定向到本机
                        let res = format!("{tcpPrefix}add rule ip self-nat PREROUTING tcp dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\
                            {udpPrefix}add rule ip self-nat PREROUTING udp dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\n\
                            ", cell = self, localPort = src_port, remotePort = dst_port, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
                        Ok(res)
                    }
                    _ => {
                        // 转发到其他机器
                        let res = format!("{tcpPrefix}add rule ip self-nat PREROUTING tcp dport {localPort} counter dnat to {dstIp}:{dstPort}  comment \"{cell}\"\n\
                            {udpPrefix}add rule ip self-nat PREROUTING udp dport {localPort} counter dnat to {dstIp}:{dstPort}  comment \"{cell}\"\n\
                            {tcpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} tcp dport {dstPort} counter {snat_to_part} comment \"{cell}\"\n\
                            {udpPrefix}add rule ip self-nat POSTROUTING ip daddr {dstIp} udp dport {dstPort} counter {snat_to_part} comment \"{cell}\"\n\n\
                            ", cell = self, localPort = src_port, dstPort = dst_port, dstIp = dst_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
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

    fn build_ipv6_rules(&self, dst_ip: &str) -> Result<String, io::Error> {
        // 从环境变量读取本机IPv6或自动探测
        let local_ipv6 = env::var("nat_local_ipv6");
        let snat_to_part = match local_ipv6 {
            Ok(ip) => "snat to ".to_owned() + &ip,
            Err(_) => "masquerade".to_owned(),
        };

        match &self {
            NatCell::Range {
                port_start,
                port_end,
                dst_domain: _,
                protocol,
                ip_version: _,
            } => {
                let res=format!("{tcpPrefix}add rule ip6 self-nat PREROUTING tcp dport {portStart}-{portEnd} counter dnat to [{dstIp}]:{portStart}-{portEnd} comment \"{cell}\"\n\
                    {udpPrefix}add rule ip6 self-nat PREROUTING udp dport {portStart}-{portEnd} counter dnat to [{dstIp}]:{portStart}-{portEnd} comment \"{cell}\"\n\
                    {tcpPrefix}add rule ip6 self-nat POSTROUTING ip6 daddr {dstIp} tcp dport {portStart}-{portEnd} counter {snat_to_part} comment \"{cell}\"\n\
                    {udpPrefix}add rule ip6 self-nat POSTROUTING ip6 daddr {dstIp} udp dport {portStart}-{portEnd} counter {snat_to_part} comment \"{cell}\"\n\n\
                    ", cell = self, portStart = port_start, portEnd = port_end, dstIp = dst_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
                Ok(res)
            }
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
                ip_version: _,
            } => {
                match dst_domain.as_str() {
                    "localhost" | "::1" => {
                        // 重定向到本机IPv6
                        let res = format!("{tcpPrefix}add rule ip6 self-nat PREROUTING tcp dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\
                            {udpPrefix}add rule ip6 self-nat PREROUTING udp dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\n\
                            ", cell = self, localPort = src_port, remotePort = dst_port, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
                        Ok(res)
                    }
                    _ => {
                        // 转发到其他机器
                        let res = format!("{tcpPrefix}add rule ip6 self-nat PREROUTING tcp dport {localPort} counter dnat to [{dstIp}]:{dstPort}  comment \"{cell}\"\n\
                            {udpPrefix}add rule ip6 self-nat PREROUTING udp dport {localPort} counter dnat to [{dstIp}]:{dstPort}  comment \"{cell}\"\n\
                            {tcpPrefix}add rule ip6 self-nat POSTROUTING ip6 daddr {dstIp} tcp dport {dstPort} counter {snat_to_part} comment \"{cell}\"\n\
                            {udpPrefix}add rule ip6 self-nat POSTROUTING ip6 daddr {dstIp} udp dport {dstPort} counter {snat_to_part} comment \"{cell}\"\n\n\
                            ", cell = self, localPort = src_port, dstPort = dst_port, dstIp = dst_ip, tcpPrefix = protocol.tcp_prefix(), udpPrefix = protocol.udp_prefix());
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
                content: line.to_string(),
            }));
        }

        // 忽略空行
        if line.is_empty() {
            return Ok(None);
        }

        let cells: Vec<&str> = line.split(',').collect();

        // 验证字段数量 - 现在支持4-6个字段: type,port(s),domain,protocol,ip_version
        if cells.len() < 4 || cells.len() > 6 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("无效的配置行: {line}, 字段数量不正确（需要4-6个字段）"),
            ));
        }

        // 解析协议
        let protocol = if cells.len() >= 5 {
            cells[4].trim().to_string().into()
        } else {
            Protocol::All
        };

        // 解析IP版本
        let ip_version = if cells.len() >= 6 {
            cells[5].trim().to_string().into()
        } else {
            IpVersion::V4 // 默认IPv4以保持向后兼容
        };

        // 解析类型并创建NatCell
        match cells[0].trim() {
            "RANGE" => {
                let port_start = cells[1].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("无法解析起始端口: {e}"))
                })?;

                let port_end = cells[2].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("无法解析结束端口: {e}"))
                })?;

                Ok(Some(NatCell::Range {
                    port_start,
                    port_end,
                    dst_domain: cells[3].trim().to_string(),
                    protocol,
                    ip_version,
                }))
            }
            "SINGLE" => {
                let src_port = cells[1].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("无法解析源端口: {e}"))
                })?;

                let dst_port = cells[2].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("无法解析目标端口: {e}"))
                })?;

                Ok(Some(NatCell::Single {
                    src_port,
                    dst_port,
                    dst_domain: cells[3].trim().to_string(),
                    protocol,
                    ip_version,
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
        "SINGLE,10000,443,baidu.com,all,ipv4\n\
                    RANGE,1000,2000,baidu.com,tcp,ipv6\n\
                    # 格式: TYPE,port1,port2,domain,protocol,ip_version\n\
                    # TYPE: SINGLE 或 RANGE\n\
                    # protocol: tcp, udp, all\n\
                    # ip_version: ipv4, ipv6, both"
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

// 读取TOML配置文件
pub fn read_toml_config(toml_path: &str) -> Result<Vec<NatCell>, io::Error> {
    let contents = fs::read_to_string(toml_path)?;
    let config: TomlConfig = toml::from_str(&contents).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("解析TOML配置失败: {e}"))
    })?;

    let mut nat_cells = Vec::new();

    for rule in config.rules {
        match rule {
            Rule::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
                ip_version,
                comment,
            } => {
                // 如果有注释，先添加注释
                if let Some(comment_text) = comment {
                    nat_cells.push(NatCell::Comment {
                        content: format!("# {comment_text}"),
                    });
                }

                nat_cells.push(NatCell::Single {
                    src_port,
                    dst_port,
                    dst_domain,
                    protocol: protocol.into(),
                    ip_version: ip_version.into(),
                });
            }
            Rule::Range {
                port_start,
                port_end,
                dst_domain,
                protocol,
                ip_version,
                comment,
            } => {
                // 如果有注释，先添加注释
                if let Some(comment_text) = comment {
                    nat_cells.push(NatCell::Comment {
                        content: format!("# {comment_text}"),
                    });
                }

                nat_cells.push(NatCell::Range {
                    port_start,
                    port_end,
                    dst_domain,
                    protocol: protocol.into(),
                    ip_version: ip_version.into(),
                });
            }
        }
    }

    Ok(nat_cells)
}

// TOML配置示例函数
pub fn toml_example(conf: &str) -> Result<(), io::Error> {
    let example_config = TomlConfig {
        rules: vec![
            Rule::Single {
                src_port: 10000,
                dst_port: 443,
                dst_domain: "baidu.com".to_string(),
                protocol: "all".to_string(),
                ip_version: "ipv4".to_string(),
                comment: Some("百度HTTPS服务转发示例".to_string()),
            },
            Rule::Range {
                port_start: 1000,
                port_end: 2000,
                dst_domain: "baidu.com".to_string(),
                protocol: "tcp".to_string(),
                ip_version: "ipv4".to_string(),
                comment: Some("端口范围转发示例".to_string()),
            },
        ],
    };

    let toml_str = toml::to_string_pretty(&example_config)
        .map_err(|e| io::Error::other(format!("序列化TOML失败: {e}")))?;

    info!("请在 {} 编写转发规则，内容类似：\n {toml_str}", &conf);

    Ok(())
}

// TOML配置结构定义
#[derive(Debug, Serialize, Deserialize)]
pub struct TomlConfig {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Rule {
    #[serde(rename = "single")]
    Single {
        #[serde(rename = "sport")]
        src_port: i32,
        #[serde(rename = "dport")]
        dst_port: i32,
        #[serde(rename = "domain")]
        dst_domain: String,
        #[serde(default = "default_protocol")]
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
        #[serde(default)]
        comment: Option<String>,
    },
    #[serde(rename = "range")]
    Range {
        #[serde(rename = "portStart")]
        port_start: i32,
        #[serde(rename = "portEnd")]
        port_end: i32,
        #[serde(rename = "domain")]
        dst_domain: String,
        #[serde(default = "default_protocol")]
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
        #[serde(default)]
        comment: Option<String>,
    },
}

fn default_protocol() -> String {
    "all".to_string()
}

fn default_ip_version() -> String {
    "both".to_string()
}
