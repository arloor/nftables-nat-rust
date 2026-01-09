use nat_common::TomlConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

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

    pub fn to_string(&self) -> String {
        match self {
            ConfigFormat::Toml(content) => content.clone(),
            ConfigFormat::Legacy(lines) => lines
                .iter()
                .map(|l| l.line.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), io::Error> {
        let content = self.to_string();
        fs::write(path, content)?;
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
