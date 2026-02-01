use nat_common::{Args, TomlConfig};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::io;

const NAT_SERVICE_FILE: &str = "/lib/systemd/system/nat.service";

/// 配置类型信息
#[derive(Debug, Clone)]
pub struct ConfigInfo {
    pub is_toml: bool,
    pub config_path: String,
}

/// 获取配置信息
/// 优先使用命令行参数指定的配置文件，如果没有指定则从 NAT systemd service 检测
pub fn get_config_info(
    toml_config: Option<&str>,
    compatible_config: Option<&str>,
) -> Result<ConfigInfo, io::Error> {
    // 优先使用命令行参数
    if let Some(path) = toml_config {
        return Ok(ConfigInfo {
            is_toml: true,
            config_path: path.to_string(),
        });
    }
    if let Some(path) = compatible_config {
        return Ok(ConfigInfo {
            is_toml: false,
            config_path: path.to_string(),
        });
    }

    // 没有命令行参数，从 systemd service 检测
    detect_config_info_from_systemd()
}

/// 从 NAT systemd service 的 ExecStart 检测配置格式和路径
/// ExecStart 格式示例:
/// - Legacy: ExecStart=/usr/local/bin/nat /etc/nat.conf
/// - TOML:   ExecStart=/usr/local/bin/nat --toml /etc/nat.toml
fn detect_config_info_from_systemd() -> Result<ConfigInfo, io::Error> {
    let service_content = fs::read_to_string(NAT_SERVICE_FILE)?;

    // 查找 ExecStart 行
    let exec_start_line = service_content
        .lines()
        .find(|line| line.trim_start().starts_with("ExecStart="))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "ExecStart not found in nat.service",
            )
        })?;

    // 解析 ExecStart 行
    // 格式: ExecStart=/usr/local/bin/nat [--toml] <config_path>
    let exec_start = exec_start_line
        .trim_start()
        .strip_prefix("ExecStart=")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid ExecStart format"))?
        .trim();

    // 将 ExecStart 参数解析为 Args
    // 跳过第一个参数（二进制路径），构造命令行参数数组
    let parts: Vec<&str> = exec_start.split_whitespace().collect();
    if parts.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Empty ExecStart command",
        ));
    }

    // 构造 clap 解析用的参数数组（不包括二进制路径）
    let cli_args: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();

    // 使用 clap::Parser trait 的方法解析参数
    use clap::Parser;
    let args = match Args::try_parse_from(std::iter::once("nat".to_string()).chain(cli_args)) {
        Ok(args) => args,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse ExecStart arguments: {}", e),
            ));
        }
    };

    // 从 Args 中提取配置信息
    let (is_toml, config_path) = if let Some(toml_path) = args.toml {
        (true, toml_path)
    } else if let Some(legacy_path) = args.compatible_config_file {
        (false, legacy_path)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No config file specified in ExecStart",
        ));
    };

    Ok(ConfigInfo {
        is_toml,
        config_path,
    })
}

/// 根据配置信息读取配置文件
pub fn load_config(info: &ConfigInfo) -> Result<ConfigFormat, io::Error> {
    if info.is_toml {
        ConfigFormat::from_toml_file(&info.config_path)
    } else {
        ConfigFormat::from_legacy_file(&info.config_path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfigLine {
    pub line: String,
}

#[derive(Debug, Clone)]
pub enum ConfigFormat {
    Toml(String), // 直接存储 TOML 字符串
    Legacy(Vec<LegacyConfigLine>),
}

impl ConfigFormat {
    pub fn from_toml_file(path: &str) -> Result<Self, io::Error> {
        let content = fs::read_to_string(path)?;
        // 验证 TOML 格式
        TomlConfig::from_toml_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(ConfigFormat::Toml(content))
    }

    pub fn from_legacy_file(path: &str) -> Result<Self, io::Error> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<LegacyConfigLine> = content
            .lines()
            .map(|line| LegacyConfigLine {
                line: line.to_string(),
            })
            .collect();
        Ok(ConfigFormat::Legacy(lines))
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), io::Error> {
        let content = self.to_string();
        fs::write(path, content)?;
        Ok(())
    }
}

impl Display for ConfigFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigFormat::Toml(content) => write!(f, "{}", content),
            ConfigFormat::Legacy(lines) => {
                for line in lines {
                    writeln!(f, "{}", line.line)?;
                }
                Ok(())
            }
        }
    }
}

pub fn get_nftables_rules() -> Result<String, io::Error> {
    use std::process::Command;

    // Get IPv4 NAT rules
    let output = Command::new("/usr/sbin/nft")
        .arg("list")
        .arg("table")
        .arg("ip")
        .arg("self-nat")
        .output()?;

    let ipv4_nat_rules = String::from_utf8_lossy(&output.stdout).to_string();

    // Get IPv6 NAT rules
    let output6 = Command::new("/usr/sbin/nft")
        .arg("list")
        .arg("table")
        .arg("ip6")
        .arg("self-nat")
        .output();

    let ipv6_nat_rules = match output6 {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => "# IPv6 NAT table not found or not supported".to_string(),
    };

    // Get IPv4 Filter rules
    let filter_output = Command::new("/usr/sbin/nft")
        .arg("list")
        .arg("table")
        .arg("ip")
        .arg("self-filter")
        .output();

    let ipv4_filter_rules = match filter_output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => "# IPv4 filter table not found".to_string(),
    };

    // Get IPv6 Filter rules
    let filter_output6 = Command::new("/usr/sbin/nft")
        .arg("list")
        .arg("table")
        .arg("ip6")
        .arg("self-filter")
        .output();

    let ipv6_filter_rules = match filter_output6 {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => "# IPv6 filter table not found".to_string(),
    };

    Ok(format!(
        "# IPv4 NAT Rules (table ip self-nat)\n{}\n\n\
         # IPv6 NAT Rules (table ip6 self-nat)\n{}\n\n\
         # IPv4 Filter Rules (table ip self-filter)\n{}\n\n\
         # IPv6 Filter Rules (table ip6 self-filter)\n{}",
        ipv4_nat_rules, ipv6_nat_rules, ipv4_filter_rules, ipv6_filter_rules
    ))
}
