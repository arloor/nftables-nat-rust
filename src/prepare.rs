use std::{
    fs::File,
    io::{self, Write},
    process::Command,
};

use log::info;
use serde::{Deserialize, Serialize};

// Docker v28 set type filter hook forward chain policy drop
// we need set it to accept
pub(crate) fn check_and_prepare() -> Result<(), io::Error> {
    if let Some(prepare_script) = prepare_script()? {
        let final_prepare_script = format!("#!/usr/sbin/nft -f\n\n{prepare_script}\n");
        info!(
            "执行 nft -f {FILE_NAME_PREPARE}\n\
            {final_prepare_script}",
        );
        File::create(FILE_NAME_PREPARE)
            .and_then(|mut file| file.write_all(final_prepare_script.as_bytes()))?;
        let output = Command::new("/usr/sbin/nft")
            .arg("-f")
            .arg(FILE_NAME_PREPARE)
            .output()?;
        info!("执行结果: {}", output.status);
        log::info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        log::error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn prepare_script() -> Result<Option<String>, io::Error> {
    // 检查当前 nftables 中表、链和规则的存在情况
    let check_result = check_current_ruleset()?;

    let mut prepare_script = String::new();
    let mut needs_script = false;

    // 检查IPv4 FORWARD链策略
    if check_result.ip_forward_drop {
        prepare_script.push_str("# 修改 IPv4 type filter hook forward的默认策略为accept \n");
        prepare_script.push_str("chain ip filter FORWARD { policy accept ; }\n");
        needs_script = true;
    }

    // 检查IPv6 FORWARD链策略
    if check_result.ip6_forward_drop {
        prepare_script.push_str("# 修改 IPv6 type filter hook forward的默认策略为accept \n");
        prepare_script.push_str("chain ip6 filter FORWARD { policy accept ; }\n");
        needs_script = true;
    }

    if needs_script {
        Ok(Some(prepare_script))
    } else {
        Ok(None)
    }
}

fn check_current_ruleset() -> Result<CheckResult, io::Error> {
    let mut res = CheckResult::default();
    let output = Command::new("/usr/sbin/nft")
        .arg("-j")
        .arg("list")
        .arg("ruleset")
        .output()?;

    if !output.status.success() {
        info!("执行 nft -j list ruleset 命令失败");
        return Err(io::Error::other("执行 nft -j list ruleset 命令失败"));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let nftables_output: NftablesOutput = match serde_json::from_str(&json_str) {
        Ok(output) => output,
        Err(e) => {
            info!("解析 nft 输出的 JSON 失败: {e}");
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "解析 nft 输出的 JSON 失败",
            ));
        }
    };

    for entry in nftables_output.nftables {
        #[allow(clippy::single_match)]
        match entry {
            NftablesEntry::Chain { chain } => {
                // IPv4 FORWARD链检查
                // nft list table ip filter:
                // chain FORWARD {
                //      type filter hook forward priority filter; policy drop;
                // }
                if chain.family == "ip"
                    && chain.table == "filter"
                    && chain.name == "FORWARD"
                    && chain.r#type == Some("filter".to_string())
                    && chain.hook == Some("forward".to_string())
                    && chain.policy == Some("drop".to_string())
                {
                    info!(
                        "iptables-nft创建的IPv4 FORWARD链存在，且type=filter，hook=forward，policy=drop"
                    );
                    res.ip_forward_drop = true;
                }
                
                // IPv6 FORWARD链检查
                // nft list table ip6 filter:
                // chain FORWARD {
                //      type filter hook forward priority filter; policy drop;
                // }
                if chain.family == "ip6"
                    && chain.table == "filter"
                    && chain.name == "FORWARD"
                    && chain.r#type == Some("filter".to_string())
                    && chain.hook == Some("forward".to_string())
                    && chain.policy == Some("drop".to_string())
                {
                    info!(
                        "ip6tables-nft创建的IPv6 FORWARD链存在，且type=filter，hook=forward，policy=drop"
                    );
                    res.ip6_forward_drop = true;
                }
            }
            _ => {}
        }
    }

    Ok(res)
}

const FILE_NAME_PREPARE: &str = "/etc/nftables-nat/nat-prepare.nft";

// 用于解析 nft -j list ruleset 输出的数据结构
#[derive(Debug, Serialize, Deserialize)]
struct NftablesOutput {
    nftables: Vec<NftablesEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum NftablesEntry {
    Metainfo { metainfo: Metainfo },
    Table { table: Table },
    Chain { chain: Chain },
    Rule { rule: Rule },
}

#[derive(Debug, Serialize, Deserialize)]
struct Metainfo {
    version: String,
    release_name: String,
    json_schema_version: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct Table {
    family: String,
    name: String,
    handle: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Chain {
    family: String,
    table: String,
    name: String,
    handle: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prio: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Rule {
    family: String,
    table: String,
    chain: String,
    handle: u32,
    expr: Vec<serde_json::Value>,
}

#[derive(Default)]
struct CheckResult {
    ip_forward_drop: bool,
    ip6_forward_drop: bool,
}
