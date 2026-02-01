use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;
use std::num::ParseIntError;

pub mod logger;

/// Legacy配置解析错误
#[derive(Debug)]
pub enum ParseError {
    /// 注释或空行，应跳过
    Skip,
    /// 解析错误
    InvalidFormat(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Skip => write!(f, "跳过（注释或空行）"),
            ParseError::InvalidFormat(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<ParseIntError> for ParseError {
    fn from(e: ParseIntError) -> Self {
        ParseError::InvalidFormat(format!("端口解析失败: {}", e))
    }
}

// IP版本枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVersion {
    V4,
    V6,
    All, // 优先IPv4，如果IPv4不可用则使用IPv6
}

impl Default for IpVersion {
    fn default() -> Self {
        IpVersion::All
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

impl From<&str> for IpVersion {
    fn from(version: &str) -> Self {
        match version.to_lowercase().as_str() {
            "ipv4" => IpVersion::V4,
            "ipv6" => IpVersion::V6,
            "all" => IpVersion::All,
            _ => IpVersion::All,
        }
    }
}

impl Serialize for IpVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for IpVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(IpVersion::from(s))
    }
}

// 协议枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    All,
    Tcp,
    Udp,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::All
    }
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

impl From<String> for Protocol {
    fn from(protocol: String) -> Self {
        match protocol.to_lowercase().as_str() {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            _ => Protocol::All,
        }
    }
}

impl From<&str> for Protocol {
    fn from(protocol: &str) -> Self {
        match protocol.to_lowercase().as_str() {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            _ => Protocol::All,
        }
    }
}

impl From<Protocol> for String {
    fn from(protocol: Protocol) -> Self {
        protocol.to_string()
    }
}

impl Serialize for Protocol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Protocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Protocol::from(s))
    }
}

// TOML配置结构定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlConfig {
    pub rules: Vec<NftCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NftCell {
    #[serde(rename = "single")]
    Single {
        #[serde(rename = "sport")]
        sport: u16,
        #[serde(rename = "dport")]
        dport: u16,
        #[serde(rename = "domain")]
        domain: String,
        #[serde(default)]
        protocol: Protocol,
        #[serde(default)]
        ip_version: IpVersion,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
    #[serde(rename = "range")]
    Range {
        #[serde(rename = "port_start")]
        port_start: u16,
        #[serde(rename = "port_end")]
        port_end: u16,
        #[serde(rename = "domain")]
        domain: String,
        #[serde(default)]
        protocol: Protocol,
        #[serde(default)]
        ip_version: IpVersion,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
    #[serde(rename = "redirect")]
    Redirect {
        #[serde(rename = "sport")]
        src_port: u16,
        #[serde(rename = "sport_end", skip_serializing_if = "Option::is_none")]
        src_port_end: Option<u16>,
        #[serde(rename = "dport")]
        dst_port: u16,
        #[serde(default)]
        protocol: Protocol,
        #[serde(default)]
        ip_version: IpVersion,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
}

impl Display for NftCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NftCell::Single {
                sport,
                dport,
                domain,
                protocol,
                ip_version,
                ..
            } => write!(
                f,
                "SINGLE,{sport},{dport},{domain},{protocol},{ip_version}"
            ),
            NftCell::Range {
                port_start,
                port_end,
                domain,
                protocol,
                ip_version,
                ..
            } => write!(
                f,
                "RANGE,{port_start},{port_end},{domain},{protocol},{ip_version}"
            ),
            NftCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                protocol,
                ip_version,
                ..
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
        }
    }
}

impl TomlConfig {
    /// 验证配置是否合法
    pub fn validate(&self) -> Result<(), String> {
        for (idx, rule) in self.rules.iter().enumerate() {
            rule.validate()
                .map_err(|e| format!("规则 {} 验证失败: {}", idx + 1, e))?;
        }
        Ok(())
    }

    /// 从 TOML 字符串解析配置并验证
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        let config: TomlConfig = toml::from_str(s).map_err(|e| format!("解析TOML失败: {}", e))?;
        config.validate()?;
        Ok(config)
    }

    /// 转换为 TOML 字符串
    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| format!("序列化TOML失败: {}", e))
    }
}

impl TryFrom<&str> for NftCell {
    type Error = ParseError;

    /// 从legacy格式行解析NftCell
    /// 注释行和空行返回 Err(ParseError::Skip)
    /// 格式错误返回 Err(ParseError::InvalidFormat)
    fn try_from(line: &str) -> Result<Self, Self::Error> {
        let line = line.trim();

        // 处理注释和空行
        if line.is_empty() || line.starts_with('#') {
            return Err(ParseError::Skip);
        }

        let cells: Vec<&str> = line.split(',').collect();
        let rule_type = cells.first().map(|s| s.trim()).unwrap_or("");

        // 验证字段数量
        match rule_type {
            "REDIRECT" => {
                if cells.len() < 3 || cells.len() > 5 {
                    return Err(ParseError::InvalidFormat(format!(
                        "无效的配置行: {line}, REDIRECT类型需要3-5个字段"
                    )));
                }
            }
            "SINGLE" | "RANGE" => {
                if cells.len() < 4 || cells.len() > 6 {
                    return Err(ParseError::InvalidFormat(format!(
                        "无效的配置行: {line}, 字段数量不正确（需要4-6个字段）"
                    )));
                }
            }
            _ => {
                return Err(ParseError::InvalidFormat(format!(
                    "无效的转发规则类型: {}",
                    rule_type
                )));
            }
        }

        // 解析协议
        let protocol: Protocol = if rule_type == "REDIRECT" {
            if cells.len() >= 4 {
                cells[3].trim().into()
            } else {
                Protocol::All
            }
        } else if cells.len() >= 5 {
            cells[4].trim().into()
        } else {
            Protocol::All
        };

        // 解析IP版本
        let ip_version: IpVersion = if rule_type == "REDIRECT" {
            if cells.len() >= 5 {
                cells[4].trim().into()
            } else {
                IpVersion::V4 // 默认IPv4以保持向后兼容
            }
        } else if cells.len() >= 6 {
            cells[5].trim().into()
        } else {
            IpVersion::V4 // 默认IPv4以保持向后兼容
        };

        // 解析类型并创建NftCell
        match rule_type {
            "RANGE" => {
                let port_start = cells[1].trim().parse::<u16>()?;
                let port_end = cells[2].trim().parse::<u16>()?;

                Ok(NftCell::Range {
                    port_start,
                    port_end,
                    domain: cells[3].trim().to_string(),
                    protocol,
                    ip_version,
                    comment: None,
                })
            }
            "SINGLE" => {
                let sport = cells[1].trim().parse::<u16>()?;
                let dport = cells[2].trim().parse::<u16>()?;

                Ok(NftCell::Single {
                    sport,
                    dport,
                    domain: cells[3].trim().to_string(),
                    protocol,
                    ip_version,
                    comment: None,
                })
            }
            "REDIRECT" => {
                let port_field = cells[1].trim();
                let (src_port, src_port_end) = if port_field.contains('-') {
                    let parts: Vec<&str> = port_field.split('-').collect();
                    if parts.len() != 2 {
                        return Err(ParseError::InvalidFormat(format!(
                            "无效的端口范围格式: {port_field}，应为 start-end"
                        )));
                    }
                    let start = parts[0].trim().parse::<u16>()?;
                    let end = parts[1].trim().parse::<u16>()?;
                    (start, Some(end))
                } else {
                    (port_field.parse::<u16>()?, None)
                };

                let dst_port = cells[2].trim().parse::<u16>()?;

                Ok(NftCell::Redirect {
                    src_port,
                    src_port_end,
                    dst_port,
                    protocol,
                    ip_version,
                    comment: None,
                })
            }
            _ => Err(ParseError::InvalidFormat(format!(
                "无效的转发规则类型: {}",
                rule_type
            ))),
        }
    }
}

impl NftCell {
    /// 验证单个规则是否合法
    pub fn validate(&self) -> Result<(), String> {
        match self {
            NftCell::Single {
                sport,
                dport,
                domain,
                ..
            } => {
                if domain.trim().is_empty() {
                    return Err("域名不能为空".to_string());
                }
                validate_port(*sport)?;
                validate_port(*dport)?;
            }
            NftCell::Range {
                port_start,
                port_end,
                domain,
                ..
            } => {
                if domain.trim().is_empty() {
                    return Err("域名不能为空".to_string());
                }
                if port_start >= port_end {
                    return Err(format!(
                        "起始端口 {} 必须小于结束端口 {}",
                        port_start, port_end
                    ));
                }
                validate_port(*port_start)?;
                validate_port(*port_end)?;
            }
            NftCell::Redirect {
                src_port,
                src_port_end,
                dst_port,
                ..
            } => {
                if let Some(end) = src_port_end {
                    if src_port >= end {
                        return Err(format!("起始端口 {} 必须小于结束端口 {}", src_port, end));
                    }
                    validate_port(*end)?;
                }
                validate_port(*src_port)?;
                validate_port(*dst_port)?;
            }
        }
        Ok(())
    }
}

fn validate_port(port: u16) -> Result<(), String> {
    if port == 0 {
        return Err("端口号不能为0".to_string());
    }
    Ok(())
}

/// 验证legacy格式配置内容
/// 返回第一个遇到的错误，跳过注释和空行
pub fn validate_legacy_config(content: &str) -> Result<(), String> {
    for (line_num, line) in content.lines().enumerate() {
        match NftCell::try_from(line) {
            Ok(cell) => {
                cell.validate()
                    .map_err(|e| format!("第 {} 行验证失败: {}", line_num + 1, e))?;
            }
            Err(ParseError::Skip) => continue,
            Err(ParseError::InvalidFormat(msg)) => {
                return Err(format!("第 {} 行解析失败: {}", line_num + 1, msg));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_single_rule() {
        let rule = NftCell::Single {
            sport: 10000,
            dport: 443,
            domain: "example.com".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        };
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_domain() {
        let rule = NftCell::Single {
            sport: 10000,
            dport: 443,
            domain: "".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        };
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_validate_range_rule() {
        let rule = NftCell::Range {
            port_start: 1000,
            port_end: 2000,
            domain: "example.com".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::All,
            comment: None,
        };
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_range() {
        let rule = NftCell::Range {
            port_start: 2000,
            port_end: 1000,
            domain: "example.com".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        };
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_parse_and_validate_toml() {
        let toml_str = r#"
[[rules]]
type = "single"
sport = 10000
dport = 443
domain = "example.com"
protocol = "tcp"
ip_version = "ipv4"

[[rules]]
type = "range"
port_start = 1000
port_end = 2000
domain = "example.com"
protocol = "all"
ip_version = "all"
"#;
        let result = TomlConfig::from_toml_str(toml_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ip_version_serde() {
        assert_eq!(IpVersion::from("ipv4"), IpVersion::V4);
        assert_eq!(IpVersion::from("ipv6"), IpVersion::V6);
        assert_eq!(IpVersion::from("all"), IpVersion::All);
        assert_eq!(IpVersion::from("unknown"), IpVersion::All);
    }

    #[test]
    fn test_protocol_serde() {
        assert_eq!(Protocol::from("tcp"), Protocol::Tcp);
        assert_eq!(Protocol::from("udp"), Protocol::Udp);
        assert_eq!(Protocol::from("all"), Protocol::All);
        assert_eq!(Protocol::from("unknown"), Protocol::All);
    }

    #[test]
    fn test_nft_cell_display() {
        let cell = NftCell::Single {
            sport: 10000,
            dport: 443,
            domain: "example.com".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        };
        assert_eq!(cell.to_string(), "SINGLE,10000,443,example.com,tcp,ipv4");

        let cell = NftCell::Redirect {
            src_port: 8000,
            src_port_end: Some(9000),
            dst_port: 3128,
            protocol: Protocol::All,
            ip_version: IpVersion::All,
            comment: None,
        };
        assert_eq!(cell.to_string(), "REDIRECT,8000-9000,3128,all,all");
    }

    #[test]
    fn test_try_from_single() {
        let line = "SINGLE,10000,443,example.com,tcp,ipv4";
        let cell = NftCell::try_from(line).unwrap();
        match cell {
            NftCell::Single { sport, dport, domain, protocol, ip_version, .. } => {
                assert_eq!(sport, 10000);
                assert_eq!(dport, 443);
                assert_eq!(domain, "example.com");
                assert_eq!(protocol, Protocol::Tcp);
                assert_eq!(ip_version, IpVersion::V4);
            }
            _ => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_try_from_redirect_range() {
        let line = "REDIRECT,30001-39999,45678,tcp,ipv4";
        let cell = NftCell::try_from(line).unwrap();
        match cell {
            NftCell::Redirect { src_port, src_port_end, dst_port, .. } => {
                assert_eq!(src_port, 30001);
                assert_eq!(src_port_end, Some(39999));
                assert_eq!(dst_port, 45678);
            }
            _ => panic!("Expected Redirect variant"),
        }
    }

    #[test]
    fn test_try_from_comment() {
        let line = "# This is a comment";
        let result = NftCell::try_from(line);
        assert!(matches!(result, Err(ParseError::Skip)));
    }

    #[test]
    fn test_try_from_empty() {
        let line = "   ";
        let result = NftCell::try_from(line);
        assert!(matches!(result, Err(ParseError::Skip)));
    }

    #[test]
    fn test_try_from_invalid() {
        let line = "INVALID,123,456";
        let result = NftCell::try_from(line);
        assert!(matches!(result, Err(ParseError::InvalidFormat(_))));
    }

    #[test]
    fn test_validate_legacy_config() {
        let content = "# Comment\nSINGLE,10000,443,example.com,tcp,ipv4\nREDIRECT,8000,3128\n";
        assert!(validate_legacy_config(content).is_ok());
    }

    #[test]
    fn test_validate_legacy_config_invalid() {
        let content = "SINGLE,10000,443,example.com\nINVALID,123";
        let result = validate_legacy_config(content);
        assert!(result.is_err());
    }
}
