#![deny(warnings)]
use crate::ip;
use log::info;
use nat_common::{Chain, IpVersion, NftCell, ParseError, Protocol, TomlConfig};
use std::env;
use std::fmt::Display;
use std::fs;
use std::io;

/// 运行时Cell，包装NftCell和Comment
/// Comment仅用于运行时表示，不进入TOML配置
#[derive(Debug)]
pub enum RuntimeCell {
    Rule(NftCell),
    Comment(String),
}

impl Display for RuntimeCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeCell::Rule(cell) => write!(f, "{}", cell),
            RuntimeCell::Comment(content) => write!(f, "{}", content),
        }
    }
}

/// Protocol扩展trait，提供nftables专用方法
pub trait ProtocolExt {
    fn nft_proto(&self) -> &str;
}

impl ProtocolExt for Protocol {
    /// 返回nft规则中的协议部分
    /// all类型返回"meta l4proto { tcp, udp } th"，匹配所有传输层协议
    /// tcp/udp返回对应的协议名
    fn nft_proto(&self) -> &str {
        match self {
            Protocol::All => "meta l4proto { tcp, udp } th",
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
    }
}

/// NftCell构建扩展trait，提供nftables规则构建方法
pub trait NftCellBuilder {
    fn build(&self) -> Result<String, io::Error>;
}

impl NftCellBuilder for NftCell {
    fn build(&self) -> Result<String, io::Error> {
        match self {
            NftCell::Drop { .. } => build_drop_rule(self),
            _ => {
                let (domain, ip_version) = match &self {
                    NftCell::Single {
                        domain,
                        ip_version,
                        ..
                    } => (domain, ip_version),
                    NftCell::Range {
                        domain,
                        ip_version,
                        ..
                    } => (domain, ip_version),
                    NftCell::Redirect { ip_version, .. } => {
                        // Redirect doesn't need domain resolution
                        return build_redirect_rules(self, ip_version);
                    }
                    NftCell::Drop { .. } => unreachable!(),
                };

                // 根据配置的IP版本解析目标IP
                let dst_ip = ip::remote_ip(domain, ip_version)?;

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
                        result += &build_nat_rules(self, &dst_ip, &IpVersion::V4)?;
                    }
                    IpVersion::V6 => {
                        if !is_ipv6_target {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "IPv4 target address resolved but rule is configured for IPv6 only",
                            ));
                        }
                        result += &build_nat_rules(self, &dst_ip, &IpVersion::V6)?;
                    }
                    IpVersion::All => {
                        if is_ipv6_target {
                            result += &build_nat_rules(self, &dst_ip, &IpVersion::V6)?;
                        } else {
                            result += &build_nat_rules(self, &dst_ip, &IpVersion::V4)?;
                        }
                    }
                }

                Ok(result)
            }
        }
    }
}

impl RuntimeCell {
    pub fn build(&self) -> Result<String, io::Error> {
        match self {
            RuntimeCell::Rule(cell) => cell.build(),
            RuntimeCell::Comment(content) => Ok(content.clone() + "\n"),
        }
    }
}

/// 构建过滤规则的nftables脚本
fn build_drop_rule(cell: &NftCell) -> Result<String, io::Error> {
    let NftCell::Drop {
        chain,
        src_ip,
        dst_ip,
        src_port,
        src_port_end,
        dst_port,
        dst_port_end,
        protocol,
        ip_version,
        comment,
    } = cell else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected Drop cell",
        ));
    };

    let mut result = String::new();

    match ip_version {
        IpVersion::All => {
            result += &build_drop_rule_for_family(cell, chain, src_ip, dst_ip, src_port, src_port_end, dst_port, dst_port_end, protocol, comment, &IpVersion::V4)?;
            result += &build_drop_rule_for_family(cell, chain, src_ip, dst_ip, src_port, src_port_end, dst_port, dst_port_end, protocol, comment, &IpVersion::V6)?;
        }
        _ => {
            result += &build_drop_rule_for_family(cell, chain, src_ip, dst_ip, src_port, src_port_end, dst_port, dst_port_end, protocol, comment, ip_version)?;
        }
    }

    Ok(result)
}

/// 为特定IP family构建过滤规则
#[allow(clippy::too_many_arguments)]
fn build_drop_rule_for_family(
    cell: &NftCell,
    chain: &Chain,
    src_ip: &Option<String>,
    dst_ip: &Option<String>,
    src_port: &Option<u16>,
    src_port_end: &Option<u16>,
    dst_port: &Option<u16>,
    dst_port_end: &Option<u16>,
    protocol: &Protocol,
    comment: &Option<String>,
    ip_version: &IpVersion,
) -> Result<String, io::Error> {
    let (family, ip_prefix) = match ip_version {
        IpVersion::V4 => ("ip", "ip"),
        IpVersion::V6 => ("ip6", "ip6"),
        IpVersion::All => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IpVersion::All should be handled at caller level",
            ));
        }
    };

    let chain_name = match chain {
        Chain::Input => "INPUT",
        Chain::Forward => "FORWARD",
    };

    let mut conditions = Vec::new();

    // 添加协议条件
    if *protocol != Protocol::All || src_port.is_some() || dst_port.is_some() {
        let proto = protocol.nft_proto();
        conditions.push(proto.to_string());
    }

    // 添加源IP条件
    if let Some(ip) = src_ip {
        conditions.push(format!("{} saddr {}", ip_prefix, ip));
    }

    // 添加目标IP条件
    if let Some(ip) = dst_ip {
        conditions.push(format!("{} daddr {}", ip_prefix, ip));
    }

    // 添加源端口条件
    if let Some(port) = src_port {
        if let Some(end) = src_port_end {
            conditions.push(format!("sport {}-{}", port, end));
        } else {
            conditions.push(format!("sport {}", port));
        }
    }

    // 添加目标端口条件
    if let Some(port) = dst_port {
        if let Some(end) = dst_port_end {
            conditions.push(format!("dport {}-{}", port, end));
        } else {
            conditions.push(format!("dport {}", port));
        }
    }

    let conditions_str = conditions.join(" ");
    let comment_str = if let Some(cmt) = comment {
        format!(" comment \"{}\"", cmt)
    } else {
        format!(" comment \"{}\"", cell)
    };

    let rule = format!(
        "add rule {family} self-filter {chain_name} {conditions_str} counter drop{comment_str}\n\n"
    );

    Ok(rule)
}

fn build_nat_rules(cell: &NftCell, dst_ip: &str, ip_version: &IpVersion) -> Result<String, io::Error> {
    let (family, env_var, localhost_addr, fmt_ip) = match ip_version {
        IpVersion::V4 => ("ip", "nat_local_ip", "127.0.0.1", dst_ip.to_string()),
        IpVersion::V6 => ("ip6", "nat_local_ipv6", "::1", format!("[{}]", dst_ip)),
        IpVersion::All => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IpVersion::All should be handled at caller level",
            ));
        }
    };

    let snat_to_part = match env::var(env_var) {
        Ok(ip) => "snat to ".to_owned() + &ip,
        Err(_) => "masquerade".to_owned(),
    };

    match cell {
        NftCell::Range {
            port_start,
            port_end,
            protocol,
            ..
        } => {
            let proto = protocol.nft_proto();
            let res = format!(
                "add rule {family} self-nat PREROUTING ct state new {proto} dport {port_start}-{port_end} counter dnat to {fmt_ip}:{port_start}-{port_end} comment \"{cell}\"\n\
                add rule {family} self-nat POSTROUTING ct state new {family} daddr {dst_ip} {proto} dport {port_start}-{port_end} counter {snat_to_part} comment \"{cell}\"\n\n\
                ",
            );
            Ok(res)
        }
        NftCell::Single {
            sport,
            dport,
            domain,
            protocol,
            ..
        } => {
            let proto = protocol.nft_proto();
            let is_localhost = domain == "localhost" || domain == localhost_addr;
            if is_localhost {
                // 重定向到本机
                let res = format!(
                    "add rule {family} self-nat PREROUTING ct state new {proto} dport {sport} redirect to :{dport}  comment \"{cell}\"\n\n\
                    ",
                );
                Ok(res)
            } else {
                // 转发到其他机器
                let res = format!(
                    "add rule {family} self-nat PREROUTING ct state new {proto} dport {sport} counter dnat to {fmt_ip}:{dport}  comment \"{cell}\"\n\
                    add rule {family} self-nat POSTROUTING ct state new {family} daddr {dst_ip} {proto} dport {dport} counter {snat_to_part} comment \"{cell}\"\n\n\
                    ",
                );
                Ok(res)
            }
        }
        NftCell::Redirect { .. } => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Redirect cell should be built via build_redirect_rules",
        )),
        NftCell::Drop { .. } => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Drop cell should be built via build_drop_rule",
        )),
    }
}

fn build_redirect_rules(cell: &NftCell, ip_version: &IpVersion) -> Result<String, io::Error> {
    let mut result = String::new();

    match ip_version {
        IpVersion::All => {
            result += &build_redirect_rule(cell, &IpVersion::V4)?;
            result += &build_redirect_rule(cell, &IpVersion::V6)?;
        }
        _ => {
            result += &build_redirect_rule(cell, ip_version)?;
        }
    }

    Ok(result)
}

fn build_redirect_rule(cell: &NftCell, ip_version: &IpVersion) -> Result<String, io::Error> {
    let family = match ip_version {
        IpVersion::V4 => "ip",
        IpVersion::V6 => "ip6",
        IpVersion::All => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IP version for redirect rule cannot be All",
            ));
        }
    };
    match cell {
        NftCell::Redirect {
            src_port,
            src_port_end,
            dst_port,
            protocol,
            ..
        } => {
            let proto = protocol.nft_proto();
            let res = if let Some(end) = src_port_end {
                // Range redirect
                format!(
                    "add rule {family} self-nat PREROUTING ct state new {proto} dport {src_port}-{src_port_end} redirect to :{dst_port} comment \"{cell}\"\n\n\
                    ",
                    src_port_end = end,
                )
            } else {
                // Single port redirect
                format!(
                    "add rule {family} self-nat PREROUTING ct state new {proto} dport {src_port} redirect to :{dst_port} comment \"{cell}\"\n\n\
                    ",
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

/// 解析一行legacy配置，返回RuntimeCell或错误
/// 注释行返回 Some(RuntimeCell::Comment)
/// 空行返回 None
/// 规则行返回 Some(RuntimeCell::Rule)
fn parse_legacy_line(line: &str) -> Option<RuntimeCell> {
    let line = line.trim();

    // 处理注释
    if line.starts_with('#') {
        return Some(RuntimeCell::Comment(line.to_string()));
    }

    // 使用 nat-common 的 TryFrom 解析（包括NAT规则和Filter规则）
    match NftCell::try_from(line) {
        Ok(cell) => Some(RuntimeCell::Rule(cell)),
        Err(ParseError::Skip) => None,
        Err(ParseError::InvalidFormat(msg)) => {
            log::warn!("跳过无效配置行: {}", msg);
            None
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
                    REDIRECT,8000-9000,3128,tcp,all\n\
                    DROP,input,src_ip=180.213.132.211,all,ipv4\n\
                    DROP,input,src_ip=240e:328:1301::/48,all,ipv6\n\
                    DROP,forward,dst_port=22,tcp,all\n\
                    # 格式: TYPE,port(s),port/domain,protocol,ip_version\n\
                    # TYPE: SINGLE, RANGE, REDIRECT 或 DROP\n\
                    # REDIRECT格式: REDIRECT,src_port,dst_port 或 REDIRECT,src_port-src_port_end,dst_port\n\
                    # DROP格式: DROP,chain,key=value,...,protocol,ip_version\n\
                    #   chain: input 或 forward\n\
                    #   key=value: src_ip=IP, dst_ip=IP, src_port=PORT, dst_port=PORT\n\
                    # protocol: tcp, udp, all\n\
                    # ip_version: ipv4, ipv6, all"
    )
}

pub fn read_config(conf: &str) -> Result<Vec<RuntimeCell>, io::Error> {
    let mut cells = vec![];
    let mut contents = fs::read_to_string(conf)?;
    contents = contents.replace("\r\n", "\n");

    for line in contents.lines() {
        if let Some(cell) = parse_legacy_line(line) {
            cells.push(cell);
        }
    }
    Ok(cells)
}

// 读取TOML配置文件
pub fn read_toml_config(toml_path: &str) -> Result<Vec<RuntimeCell>, io::Error> {
    let contents = fs::read_to_string(toml_path)?;

    // 使用 nat-common 的解析和验证
    let config = TomlConfig::from_toml_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut cells = Vec::new();

    // 处理所有规则（包括NAT和Filter）
    for rule in config.rules {
        // 如果有注释，先添加注释
        let comment = match &rule {
            NftCell::Single { comment, .. } => comment.clone(),
            NftCell::Range { comment, .. } => comment.clone(),
            NftCell::Redirect { comment, .. } => comment.clone(),
            NftCell::Drop { comment, .. } => comment.clone(),
        };

        if let Some(comment_text) = comment {
            cells.push(RuntimeCell::Comment(format!("# {comment_text}")));
        }

        cells.push(RuntimeCell::Rule(rule));
    }

    Ok(cells)
}

// TOML配置示例函数
pub fn toml_example(conf: &str) -> Result<(), io::Error> {
    let example_config = TomlConfig {
        rules: vec![
            NftCell::Single {
                sport: 10000,
                dport: 443,
                domain: "baidu.com".to_string(),
                protocol: Protocol::All,
                ip_version: IpVersion::V4,
                comment: Some("百度HTTPS服务转发示例".to_string()),
            },
            NftCell::Range {
                port_start: 1000,
                port_end: 2000,
                domain: "baidu.com".to_string(),
                protocol: Protocol::Tcp,
                ip_version: IpVersion::V4,
                comment: Some("端口范围转发示例".to_string()),
            },
            NftCell::Redirect {
                src_port: 8000,
                src_port_end: None,
                dst_port: 3128,
                protocol: Protocol::All,
                ip_version: IpVersion::V4,
                comment: Some("单端口重定向到本机示例".to_string()),
            },
            NftCell::Redirect {
                src_port: 30001,
                src_port_end: Some(39999),
                dst_port: 45678,
                protocol: Protocol::Tcp,
                ip_version: IpVersion::All,
                comment: Some("端口范围重定向到本机示例".to_string()),
            },
            NftCell::Drop {
                chain: Chain::Input,
                src_ip: Some("180.213.132.211".to_string()),
                dst_ip: None,
                src_port: None,
                src_port_end: None,
                dst_port: None,
                dst_port_end: None,
                protocol: Protocol::All,
                ip_version: IpVersion::V4,
                comment: Some("阻止特定IPv4地址".to_string()),
            },
            NftCell::Drop {
                chain: Chain::Input,
                src_ip: Some("240e:328:1301::/48".to_string()),
                dst_ip: None,
                src_port: None,
                src_port_end: None,
                dst_port: None,
                dst_port_end: None,
                protocol: Protocol::All,
                ip_version: IpVersion::V6,
                comment: Some("阻止IPv6网段".to_string()),
            },
            NftCell::Drop {
                chain: Chain::Input,
                src_ip: None,
                dst_ip: None,
                src_port: None,
                src_port_end: None,
                dst_port: Some(22),
                dst_port_end: None,
                protocol: Protocol::Tcp,
                ip_version: IpVersion::All,
                comment: Some("阻止SSH端口访问".to_string()),
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
        let result = parse_legacy_line(line);
        assert!(result.is_some());
        match result.unwrap() {
            RuntimeCell::Rule(NftCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                ..
            }) => {
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
        let result = parse_legacy_line(line);
        assert!(result.is_some());
        match result.unwrap() {
            RuntimeCell::Rule(NftCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                ..
            }) => {
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
        let result = parse_legacy_line(line);
        assert!(result.is_some());
        match result.unwrap() {
            RuntimeCell::Rule(NftCell::Redirect {
                src_port, dst_port, ..
            }) => {
                assert_eq!(src_port, 9000);
                assert_eq!(dst_port, 8080);
            }
            other => panic!("Expected Redirect variant, got {:?}", other),
        }
    }

    #[test]
    fn test_backward_compatibility_localhost() {
        let line = "SINGLE,2222,22,localhost";
        let result = parse_legacy_line(line);
        assert!(result.is_some());
        match result.unwrap() {
            RuntimeCell::Rule(NftCell::Single {
                sport,
                dport,
                domain,
                ..
            }) => {
                assert_eq!(sport, 2222);
                assert_eq!(dport, 22);
                assert_eq!(domain, "localhost");
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
        let cell = NftCell::Redirect {
            src_port: 8000,
            src_port_end: None,
            dst_port: 3128,
            protocol: Protocol::All,
            ip_version: IpVersion::V4,
            comment: None,
        };

        let result = cell.build().unwrap();
        // all协议使用th dport匹配所有传输层协议
        assert!(result.contains("add rule ip self-nat PREROUTING ct state new meta l4proto { tcp, udp } th dport 8000 redirect to :3128"));
        assert!(!result.contains("ip6")); // Should not have IPv6 rules
    }

    #[test]
    fn test_build_redirect_range_ipv4() {
        let cell = NftCell::Redirect {
            src_port: 30001,
            src_port_end: Some(39999),
            dst_port: 45678,
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        };

        let result = cell.build().unwrap();
        // tcp协议只生成tcp规则
        assert!(result.contains(
            "add rule ip self-nat PREROUTING ct state new tcp dport 30001-39999 redirect to :45678"
        ));
        assert!(!result.contains("udp")); // tcp协议不应该包含udp规则
        assert!(!result.contains("ip6")); // Should not have IPv6 rules
    }

    #[test]
    fn test_build_redirect_both_ipv() {
        let cell = NftCell::Redirect {
            src_port: 5000,
            src_port_end: None,
            dst_port: 4000,
            protocol: Protocol::All,
            ip_version: IpVersion::All,
            comment: None,
        };

        let result = cell.build().unwrap();
        // all协议应该使用th dport，同时包含IPv4和IPv6
        assert!(result.contains("add rule ip self-nat PREROUTING ct state new meta l4proto { tcp, udp } th dport 5000 redirect to :4000"));
        assert!(
            result.contains("add rule ip6 self-nat PREROUTING ct state new meta l4proto { tcp, udp } th dport 5000 redirect to :4000")
        );
    }
}
