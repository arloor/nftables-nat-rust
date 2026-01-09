use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Rule {
    Single {
        sport: u16,
        dport: u16,
        domain: String,
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
    Range {
        #[serde(rename = "portStart")]
        port_start: u16,
        #[serde(rename = "portEnd")]
        port_end: u16,
        domain: String,
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
    Redirect {
        #[serde(rename = "srcPort")]
        src_port: String, // 可以是 "8080" 或 "8080-8090"
        #[serde(rename = "dstPort")]
        dst_port: u16,
        protocol: String,
        #[serde(default = "default_ip_version")]
        ip_version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
    },
}

fn default_ip_version() -> String {
    "both".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlConfig {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConfigLine {
    pub line: String,
}

#[derive(Debug, Clone)]
pub enum ConfigFormat {
    Toml(TomlConfig),
    Legacy(Vec<LegacyConfigLine>),
}

impl ConfigFormat {
    pub fn from_toml_file(path: &str) -> Result<Self, io::Error> {
        let content = fs::read_to_string(path)?;
        let config: TomlConfig =
            toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(ConfigFormat::Toml(config))
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

    pub fn to_toml_string(&self) -> Result<String, io::Error> {
        match self {
            ConfigFormat::Toml(config) => toml::to_string_pretty(config)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            ConfigFormat::Legacy(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Cannot convert legacy config to TOML",
            )),
        }
    }

    pub fn to_legacy_string(&self) -> Result<String, io::Error> {
        match self {
            ConfigFormat::Legacy(lines) => Ok(lines
                .iter()
                .map(|l| l.line.clone())
                .collect::<Vec<_>>()
                .join("\n")),
            ConfigFormat::Toml(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Cannot convert TOML config to legacy",
            )),
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), io::Error> {
        match self {
            ConfigFormat::Toml(_) => {
                let content = self.to_toml_string()?;
                fs::write(path, content)?;
            }
            ConfigFormat::Legacy(_) => {
                let content = self.to_legacy_string()?;
                fs::write(path, content)?;
            }
        }
        Ok(())
    }
}

pub fn get_nftables_rules() -> Result<String, io::Error> {
    use std::process::Command;

    let output = Command::new("/usr/sbin/nft")
        .arg("list")
        .arg("table")
        .arg("ip")
        .arg("self-nat")
        .output()?;

    let ipv4_rules = String::from_utf8_lossy(&output.stdout).to_string();

    let output6 = Command::new("/usr/sbin/nft")
        .arg("list")
        .arg("table")
        .arg("ip6")
        .arg("self-nat")
        .output();

    let ipv6_rules = match output6 {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => "# IPv6 table not found or not supported".to_string(),
    };

    Ok(format!(
        "# IPv4 NAT Rules (table ip self-nat)\n{}\n\n# IPv6 NAT Rules (table ip6 self-nat)\n{}",
        ipv4_rules, ipv6_rules
    ))
}
