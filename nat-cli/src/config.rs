#![deny(warnings)]
use crate::ip;
use log::info;
use nat_common::{Rule as CommonRule, TomlConfig};
use std::env;
use std::fmt::Display;
use std::fs;
use std::io;

#[derive(Debug, Clone)]
pub enum IpVersion {
    V4,
    V6,
    All, // 优先IPv4，如果IPv4不可用则使用IPv6
}

impl From<String> for IpVersion {
    fn from(version: String) -> Self {
        match version.to_lowercase().as_str() {
            "ipv4" => IpVersion::V4,
            "ipv6" => IpVersion::V6,
            "all" => IpVersion::All,
            _ => IpVersion::All,
        }
    }
}

impl Display for IpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpVersion::V4 => write!(f, "ipv4"),
            IpVersion::V6 => write!(f, "ipv6"),
            IpVersion::All => write!(f, "all"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Protocol {
    All,
    Tcp,
    Udp,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::All => write!(f, "all"),
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Udp => write!(f, "udp"),
        }
    }
}

impl Protocol {
    // 返回nft规则中的协议部分
    // all类型返回"th"(transport header)，匹配所有传输层协议
    // tcp/udp返回对应的协议名
    fn nft_proto(&self) -> &str {
        match self {
            Protocol::All => "meta l4proto { tcp, udp } th",
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
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
        match protocol.to_lowercase().as_str() {
            "udp" => Protocol::Udp,
            "tcp" => Protocol::Tcp,
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
    Redirect {
        src_port: i32,
        src_port_end: Option<i32>, // None for single port, Some for range
        dst_port: i32,
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
                "SINGLE,{src_port},{dst_port},{dst_domain},{protocol},{ip_version}"
            ),
            NatCell::Range {
                port_start,
                port_end,
                dst_domain,
                protocol,
                ip_version,
            } => write!(
                f,
                "RANGE,{port_start},{port_end},{dst_domain},{protocol},{ip_version}"
            ),
            NatCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                protocol,
                ip_version,
            } => {
                if let Some(end) = src_port_end {
                    write!(
                        f,
                        "REDIRECT,{src_port}-{end},{dst_port},{protocol},{ip_version}"
                    )
                } else {
                    write!(f, "REDIRECT,{src_port},{dst_port},{protocol},{ip_version}")
                }
            }
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
            NatCell::Redirect { ip_version, .. } => {
                // Redirect doesn't need domain resolution
                return self.build_redirect_rules(ip_version);
            }
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
            IpVersion::All => {
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
                let proto = protocol.nft_proto();
                let res = format!(
                    "add rule ip self-nat PREROUTING ct state new {proto} dport {portStart}-{portEnd} counter dnat to {dstIp}:{portStart}-{portEnd} comment \"{cell}\"\n\
                    add rule ip self-nat POSTROUTING ct state new ip daddr {dstIp} {proto} dport {portStart}-{portEnd} counter {snat_to_part} comment \"{cell}\"\n\n\
                    ",
                    cell = self,
                    portStart = port_start,
                    portEnd = port_end,
                    dstIp = dst_ip,
                );
                Ok(res)
            }
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
                ip_version: _,
            } => {
                let proto = protocol.nft_proto();
                match dst_domain.as_str() {
                    "localhost" | "127.0.0.1" => {
                        // 重定向到本机
                        let res = format!(
                            "add rule ip self-nat PREROUTING ct state new {proto} dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\n\
                            ",
                            cell = self,
                            localPort = src_port,
                            remotePort = dst_port,
                        );
                        Ok(res)
                    }
                    _ => {
                        // 转发到其他机器
                        let res = format!(
                            "add rule ip self-nat PREROUTING ct state new {proto} dport {localPort} counter dnat to {dstIp}:{dstPort}  comment \"{cell}\"\n\
                            add rule ip self-nat POSTROUTING ct state new ip daddr {dstIp} {proto} dport {dstPort} counter {snat_to_part} comment \"{cell}\"\n\n\
                            ",
                            cell = self,
                            localPort = src_port,
                            dstPort = dst_port,
                            dstIp = dst_ip,
                        );
                        Ok(res)
                    }
                }
            }
            NatCell::Comment { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Comment cell cannot be built",
            )),
            NatCell::Redirect { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Redirect cell should be built via build_redirect_rules",
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
                let proto = protocol.nft_proto();
                let res = format!(
                    "add rule ip6 self-nat PREROUTING ct state new {proto} dport {portStart}-{portEnd} counter dnat to [{dstIp}]:{portStart}-{portEnd} comment \"{cell}\"\n\
                    add rule ip6 self-nat POSTROUTING ct state new ip6 daddr {dstIp} {proto} dport {portStart}-{portEnd} counter {snat_to_part} comment \"{cell}\"\n\n\
                    ",
                    cell = self,
                    portStart = port_start,
                    portEnd = port_end,
                    dstIp = dst_ip,
                );
                Ok(res)
            }
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                protocol,
                ip_version: _,
            } => {
                let proto = protocol.nft_proto();
                match dst_domain.as_str() {
                    "localhost" | "::1" => {
                        // 重定向到本机IPv6
                        let res = format!(
                            "add rule ip6 self-nat PREROUTING ct state new {proto} dport {localPort} redirect to :{remotePort}  comment \"{cell}\"\n\n\
                            ",
                            cell = self,
                            localPort = src_port,
                            remotePort = dst_port,
                        );
                        Ok(res)
                    }
                    _ => {
                        // 转发到其他机器
                        let res = format!(
                            "add rule ip6 self-nat PREROUTING ct state new {proto} dport {localPort} counter dnat to [{dstIp}]:{dstPort}  comment \"{cell}\"\n\
                            add rule ip6 self-nat POSTROUTING ct state new ip6 daddr {dstIp} {proto} dport {dstPort} counter {snat_to_part} comment \"{cell}\"\n\n\
                            ",
                            cell = self,
                            localPort = src_port,
                            dstPort = dst_port,
                            dstIp = dst_ip,
                        );
                        Ok(res)
                    }
                }
            }
            NatCell::Comment { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Comment cell cannot be built",
            )),
            NatCell::Redirect { .. } => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Redirect cell should be built via build_redirect_rules",
            )),
        }
    }

    fn build_redirect_rules(&self, ip_version: &IpVersion) -> Result<String, io::Error> {
        let mut result = String::new();

        match ip_version {
            IpVersion::V4 => {
                result += &self.build_ipv4_redirect_rules()?;
            }
            IpVersion::V6 => {
                result += &self.build_ipv6_redirect_rules()?;
            }
            IpVersion::All => {
                result += &self.build_ipv4_redirect_rules()?;
                result += &self.build_ipv6_redirect_rules()?;
            }
        }

        Ok(result)
    }

    fn build_ipv4_redirect_rules(&self) -> Result<String, io::Error> {
        match &self {
            NatCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                protocol,
                ip_version: _,
            } => {
                let proto = protocol.nft_proto();
                let res = if let Some(end) = src_port_end {
                    // Range redirect
                    format!(
                        "add rule ip self-nat PREROUTING ct state new {proto} dport {src_port}-{src_port_end} redirect to :{dst_port} comment \"{cell}\"\n\n\
                        ",
                        cell = self,
                        src_port = src_port,
                        src_port_end = end,
                        dst_port = dst_port,
                    )
                } else {
                    // Single port redirect
                    format!(
                        "add rule ip self-nat PREROUTING ct state new {proto} dport {src_port} redirect to :{dst_port} comment \"{cell}\"\n\n\
                        ",
                        cell = self,
                        src_port = src_port,
                        dst_port = dst_port,
                    )
                };
                Ok(res)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a Redirect cell",
            )),
        }
    }

    fn build_ipv6_redirect_rules(&self) -> Result<String, io::Error> {
        match &self {
            NatCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                protocol,
                ip_version: _,
            } => {
                let proto = protocol.nft_proto();
                let res = if let Some(end) = src_port_end {
                    // Range redirect
                    format!(
                        "add rule ip6 self-nat PREROUTING ct state new {proto} dport {src_port}-{src_port_end} redirect to :{dst_port} comment \"{cell}\"\n\n\
                        ",
                        cell = self,
                        src_port = src_port,
                        src_port_end = end,
                        dst_port = dst_port,
                    )
                } else {
                    // Single port redirect
                    format!(
                        "add rule ip6 self-nat PREROUTING ct state new {proto} dport {src_port} redirect to :{dst_port} comment \"{cell}\"\n\n\
                        ",
                        cell = self,
                        src_port = src_port,
                        dst_port = dst_port,
                    )
                };
                Ok(res)
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a Redirect cell",
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

        // 解析类型以确定所需的字段数量
        let rule_type = cells.first().map(|s| s.trim()).unwrap_or("");

        // 验证字段数量
        match rule_type {
            "REDIRECT" => {
                // REDIRECT,port(s),dst_port[,protocol[,ip_version]]
                // 需要3-5个字段
                if cells.len() < 3 || cells.len() > 5 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("无效的配置行: {line}, REDIRECT类型需要3-5个字段"),
                    ));
                }
            }
            "SINGLE" | "RANGE" => {
                // 需要4-6个字段: type,port(s),domain,protocol,ip_version
                if cells.len() < 4 || cells.len() > 6 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("无效的配置行: {line}, 字段数量不正确（需要4-6个字段）"),
                    ));
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("无效的转发规则类型: {}", rule_type),
                ));
            }
        }

        // 解析协议 - REDIRECT从第3个字段开始，SINGLE/RANGE从第4个字段开始
        let protocol = if rule_type == "REDIRECT" {
            if cells.len() >= 4 {
                cells[3].trim().to_string().into()
            } else {
                Protocol::All
            }
        } else if cells.len() >= 5 {
            cells[4].trim().to_string().into()
        } else {
            Protocol::All
        };

        // 解析IP版本 - REDIRECT从第4个字段开始，SINGLE/RANGE从第5个字段开始
        let ip_version = if rule_type == "REDIRECT" {
            if cells.len() >= 5 {
                cells[4].trim().to_string().into()
            } else {
                IpVersion::V4 // 默认IPv4以保持向后兼容
            }
        } else if cells.len() >= 6 {
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
            "REDIRECT" => {
                // 解析第二个字段：可能是单个端口或端口范围（格式：8000 或 8000-9000）
                let port_field = cells[1].trim();
                let (src_port, src_port_end) = if port_field.contains('-') {
                    // 端口范围
                    let parts: Vec<&str> = port_field.split('-').collect();
                    if parts.len() != 2 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("无效的端口范围格式: {port_field}，应为 start-end"),
                        ));
                    }
                    let start = parts[0].trim().parse::<i32>().map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("无法解析起始端口: {e}"))
                    })?;
                    let end = parts[1].trim().parse::<i32>().map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("无法解析结束端口: {e}"))
                    })?;
                    (start, Some(end))
                } else {
                    // 单个端口
                    let port = port_field.parse::<i32>().map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("无法解析源端口: {e}"))
                    })?;
                    (port, None)
                };

                // 解析目标端口
                let dst_port = cells[2].trim().parse::<i32>().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("无法解析目标端口: {e}"))
                })?;

                Ok(Some(NatCell::Redirect {
                    src_port,
                    src_port_end,
                    dst_port,
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
                    REDIRECT,8000,3128,all,ipv4\n\
                    REDIRECT,8000-9000,3128,tcp,both\n\
                    # 格式: TYPE,port(s),port/domain,protocol,ip_version\n\
                    # TYPE: SINGLE, RANGE, 或 REDIRECT\n\
                    # REDIRECT格式: REDIRECT,src_port,dst_port 或 REDIRECT,src_port-src_port_end,dst_port\n\
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

    // 使用 nat-common 的解析和验证
    let config = TomlConfig::from_toml_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut nat_cells = Vec::new();

    for rule in config.rules {
        match rule {
            CommonRule::Single {
                sport,
                dport,
                domain,
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
                    src_port: sport as i32,
                    dst_port: dport as i32,
                    dst_domain: domain,
                    protocol: protocol.into(),
                    ip_version: ip_version.into(),
                });
            }
            CommonRule::Range {
                port_start,
                port_end,
                domain,
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
                    port_start: port_start as i32,
                    port_end: port_end as i32,
                    dst_domain: domain,
                    protocol: protocol.into(),
                    ip_version: ip_version.into(),
                });
            }
            CommonRule::Redirect {
                src_port,
                src_port_end,
                dst_port,
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

                nat_cells.push(NatCell::Redirect {
                    src_port: src_port as i32,
                    src_port_end: src_port_end.map(|p| p as i32),
                    dst_port: dst_port as i32,
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
    use nat_common::Rule as CommonRule;

    let example_config = TomlConfig {
        rules: vec![
            CommonRule::Single {
                sport: 10000,
                dport: 443,
                domain: "baidu.com".to_string(),
                protocol: "all".to_string(),
                ip_version: "ipv4".to_string(),
                comment: Some("百度HTTPS服务转发示例".to_string()),
            },
            CommonRule::Range {
                port_start: 1000,
                port_end: 2000,
                domain: "baidu.com".to_string(),
                protocol: "tcp".to_string(),
                ip_version: "ipv4".to_string(),
                comment: Some("端口范围转发示例".to_string()),
            },
            CommonRule::Redirect {
                src_port: 8000,
                src_port_end: None,
                dst_port: 3128,
                protocol: "all".to_string(),
                ip_version: "ipv4".to_string(),
                comment: Some("单端口重定向到本机示例".to_string()),
            },
            CommonRule::Redirect {
                src_port: 30001,
                src_port_end: Some(39999),
                dst_port: 45678,
                protocol: "tcp".to_string(),
                ip_version: "all".to_string(),
                comment: Some("端口范围重定向到本机示例".to_string()),
            },
        ],
    };

    let toml_str = example_config
        .to_toml_string()
        .map_err(|e| io::Error::other(format!("序列化TOML失败: {e}")))?;

    info!("请在 {} 编写转发规则，内容类似：\n {toml_str}", &conf);

    Ok(())
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod redirect_parse_tests {
    use super::*;

    #[test]
    fn test_parse_redirect_single_port() {
        let line = "REDIRECT,8000,3128";
        let result = NatCell::parse(line).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            NatCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                ..
            } => {
                assert_eq!(src_port, 8000);
                assert_eq!(src_port_end, None);
                assert_eq!(dst_port, 3128);
            }
            other => panic!("Expected Redirect variant, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_redirect_port_range() {
        let line = "REDIRECT,30001-39999,45678";
        let result = NatCell::parse(line).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            NatCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                ..
            } => {
                assert_eq!(src_port, 30001);
                assert_eq!(src_port_end, Some(39999));
                assert_eq!(dst_port, 45678);
            }
            other => panic!("Expected Redirect variant, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_redirect_with_protocol() {
        let line = "REDIRECT,9000,8080,tcp";
        let result = NatCell::parse(line).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            NatCell::Redirect {
                src_port, dst_port, ..
            } => {
                assert_eq!(src_port, 9000);
                assert_eq!(dst_port, 8080);
            }
            other => panic!("Expected Redirect variant, got {:?}", other),
        }
    }

    #[test]
    fn test_backward_compatibility_localhost() {
        let line = "SINGLE,2222,22,localhost";
        let result = NatCell::parse(line).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            NatCell::Single {
                src_port,
                dst_port,
                dst_domain,
                ..
            } => {
                assert_eq!(src_port, 2222);
                assert_eq!(dst_port, 22);
                assert_eq!(dst_domain, "localhost");
            }
            other => panic!("Expected Single variant, got {:?}", other),
        }
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod redirect_build_tests {
    use super::*;

    #[test]
    fn test_build_redirect_single_ipv4() {
        let cell = NatCell::Redirect {
            src_port: 8000,
            src_port_end: None,
            dst_port: 3128,
            protocol: Protocol::All,
            ip_version: IpVersion::V4,
        };

        let result = cell.build().unwrap();
        // all协议使用th dport匹配所有传输层协议
        assert!(result.contains("add rule ip self-nat PREROUTING ct state new meta l4proto { tcp, udp } th dport 8000 redirect to :3128"));
        assert!(!result.contains("ip6")); // Should not have IPv6 rules
    }

    #[test]
    fn test_build_redirect_range_ipv4() {
        let cell = NatCell::Redirect {
            src_port: 30001,
            src_port_end: Some(39999),
            dst_port: 45678,
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
        };

        let result = cell.build().unwrap();
        // tcp协议只生成tcp规则
        assert!(
            result.contains(
                "add rule ip self-nat PREROUTING ct state new tcp dport 30001-39999 redirect to :45678"
            )
        );
        assert!(!result.contains("udp")); // tcp协议不应该包含udp规则
        assert!(!result.contains("ip6")); // Should not have IPv6 rules
    }

    #[test]
    fn test_build_redirect_both_ipv() {
        let cell = NatCell::Redirect {
            src_port: 5000,
            src_port_end: None,
            dst_port: 4000,
            protocol: Protocol::All,
            ip_version: IpVersion::All,
        };

        let result = cell.build().unwrap();
        // all协议应该使用th dport，同时包含IPv4和IPv6
        assert!(result.contains("add rule ip self-nat PREROUTING ct state new meta l4proto { tcp, udp } th dport 5000 redirect to :4000"));
        assert!(
            result.contains("add rule ip6 self-nat PREROUTING ct state new meta l4proto { tcp, udp } th dport 5000 redirect to :4000")
        );
    }
}
