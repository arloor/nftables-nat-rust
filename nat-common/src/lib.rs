use serde::{Deserialize, Serialize};

pub mod logger;

// TOML配置结构定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlConfig {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Rule {
    #[serde(rename = "single")]
    Single {
        #[serde(rename = "sport")]
        sport: u16,
        #[serde(rename = "dport")]
        dport: u16,
        #[serde(rename = "domain")]
        domain: String,
        #[serde(default = "default_protocol")]
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
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
        #[serde(default = "default_protocol")]
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
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
        #[serde(default = "default_protocol")]
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
}

fn default_protocol() -> String {
    "all".to_string()
}

fn default_ip_version() -> String {
    "all".to_string()
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

impl Rule {
    /// 验证单个规则是否合法
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Rule::Single {
                sport,
                dport,
                domain,
                protocol,
                ip_version,
                ..
            } => {
                if domain.trim().is_empty() {
                    return Err("域名不能为空".to_string());
                }
                validate_protocol(protocol)?;
                validate_ip_version(ip_version)?;
                validate_port(*sport)?;
                validate_port(*dport)?;
            }
            Rule::Range {
                port_start,
                port_end,
                domain,
                protocol,
                ip_version,
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
                validate_protocol(protocol)?;
                validate_ip_version(ip_version)?;
                validate_port(*port_start)?;
                validate_port(*port_end)?;
            }
            Rule::Redirect {
                src_port,
                src_port_end,
                dst_port,
                protocol,
                ip_version,
                ..
            } => {
                if let Some(end) = src_port_end {
                    if src_port >= end {
                        return Err(format!("起始端口 {} 必须小于结束端口 {}", src_port, end));
                    }
                    validate_port(*end)?;
                }
                validate_protocol(protocol)?;
                validate_ip_version(ip_version)?;
                validate_port(*src_port)?;
                validate_port(*dst_port)?;
            }
        }
        Ok(())
    }
}

fn validate_protocol(protocol: &str) -> Result<(), String> {
    match protocol.to_lowercase().as_str() {
        "tcp" | "udp" | "all" => Ok(()),
        _ => Err(format!("无效的协议: {}, 必须是 tcp, udp 或 all", protocol)),
    }
}

fn validate_ip_version(ip_version: &str) -> Result<(), String> {
    match ip_version.to_lowercase().as_str() {
        "ipv4" | "ipv6" | "all" => Ok(()),
        _ => Err(format!(
            "无效的IP版本: {}, 必须是 ipv4, ipv6 或 all",
            ip_version
        )),
    }
}

fn validate_port(port: u16) -> Result<(), String> {
    if port == 0 {
        return Err("端口号不能为0".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_single_rule() {
        let rule = Rule::Single {
            sport: 10000,
            dport: 443,
            domain: "example.com".to_string(),
            protocol: "tcp".to_string(),
            ip_version: "ipv4".to_string(),
            comment: None,
        };
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_domain() {
        let rule = Rule::Single {
            sport: 10000,
            dport: 443,
            domain: "".to_string(),
            protocol: "tcp".to_string(),
            ip_version: "ipv4".to_string(),
            comment: None,
        };
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_protocol() {
        let rule = Rule::Single {
            sport: 10000,
            dport: 443,
            domain: "example.com".to_string(),
            protocol: "http".to_string(),
            ip_version: "ipv4".to_string(),
            comment: None,
        };
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_validate_range_rule() {
        let rule = Rule::Range {
            port_start: 1000,
            port_end: 2000,
            domain: "example.com".to_string(),
            protocol: "tcp".to_string(),
            ip_version: "all".to_string(),
            comment: None,
        };
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_range() {
        let rule = Rule::Range {
            port_start: 2000,
            port_end: 1000,
            domain: "example.com".to_string(),
            protocol: "tcp".to_string(),
            ip_version: "ipv4".to_string(),
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
}
