#![allow(dead_code)]
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
            NftablesEntry::Chain {
                family,
                table,
                name,
                handle: _,
                r#type,
                hook,
                prio: _,
                policy,
            } => {
                // IPv4 FORWARD链检查
                // nft list table ip filter:
                // chain FORWARD {
                //      type filter hook forward priority filter; policy drop;
                // }
                if family == "ip"
                    && table == "filter"
                    && name == "FORWARD"
                    && r#type == Some("filter".to_string())
                    && hook == Some("forward".to_string())
                    && policy == Some("drop".to_string())
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
                if family == "ip6"
                    && table == "filter"
                    && name == "FORWARD"
                    && r#type == Some("filter".to_string())
                    && hook == Some("forward".to_string())
                    && policy == Some("drop".to_string())
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
// #[serde(untagged)]
#[serde(rename_all = "snake_case")]
enum NftablesEntry {
    Metainfo {
        version: String,
        release_name: String,
        json_schema_version: u8,
    },
    Table {
        family: String,
        name: String,
        handle: u32,
    },
    Chain {
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
    },
    Rule {
        family: String,
        table: String,
        chain: String,
        handle: u32,
        expr: Vec<serde_json::Value>,
    },
    Set {
        family: String,
        table: String,
        name: String,
        handle: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        r#type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        policy: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flags: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        elem: Option<Vec<serde_json::Value>>,
    },
    Map {
        family: String,
        table: String,
        name: String,
        handle: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        r#type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        map: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flags: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        elem: Option<Vec<serde_json::Value>>,
    },
    Element {
        family: String,
        table: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        elem: Option<Vec<serde_json::Value>>,
    },
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize)]
struct Metainfo {}

#[derive(Debug, Serialize, Deserialize)]
struct Table {}

#[derive(Debug, Serialize, Deserialize)]
struct Chain {}

#[derive(Debug, Serialize, Deserialize)]
struct Rule {}

#[derive(Debug, Serialize, Deserialize)]
struct Set {}

#[derive(Debug, Serialize, Deserialize)]
struct Map {}

#[derive(Debug, Serialize, Deserialize)]
struct Element {}

#[derive(Default)]
struct CheckResult {
    ip_forward_drop: bool,
    ip6_forward_drop: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_nftables_output() {
        let json_data = r#"{
    "nftables": [
        {
            "metainfo": {
                "version": "1.1.3",
                "release_name": "Commodore Bullmoose #4",
                "json_schema_version": 1
            }
        },
        {
            "table": {
                "family": "inet",
                "name": "filter",
                "handle": 1
            }
        },
        {
            "chain": {
                "family": "inet",
                "table": "filter",
                "name": "input",
                "handle": 1,
                "type": "filter",
                "hook": "input",
                "prio": 0,
                "policy": "accept"
            }
        },
        {
            "chain": {
                "family": "inet",
                "table": "filter",
                "name": "forward",
                "handle": 2,
                "type": "filter",
                "hook": "forward",
                "prio": 0,
                "policy": "accept"
            }
        },
        {
            "chain": {
                "family": "inet",
                "table": "filter",
                "name": "output",
                "handle": 3,
                "type": "filter",
                "hook": "output",
                "prio": 0,
                "policy": "accept"
            }
        },
        {
            "table": {
                "family": "ip",
                "name": "netbird",
                "handle": 2
            }
        },
        {
            "set": {
                "family": "ip",
                "name": "nb0000001",
                "table": "netbird",
                "type": "ipv4_addr",
                "handle": 40,
                "flags": [
                    "dynamic"
                ],
                "elem": [
                    "0.0.0.0"
                ]
            }
        },
        {
            "rule": {
                "family": "ip",
                "table": "netbird",
                "chain": "netbird-rt-fwd",
                "handle": 22,
                "expr": [
                    {
                        "match": {
                            "op": "in",
                            "left": {
                                "ct": {
                                    "key": "state"
                                }
                            },
                            "right": [
                                "established",
                                "related"
                            ]
                        }
                    },
                    {
                        "counter": {
                            "packets": 0,
                            "bytes": 0
                        }
                    },
                    {
                        "accept": null
                    }
                ]
            }
        }
    ]
}"#;

        let result: Result<NftablesOutput, _> = serde_json::from_str(json_data);
        assert!(
            result.is_ok(),
            "Failed to deserialize JSON: {:?}",
            result.err()
        );

        let nftables_output = result.unwrap();
        assert_eq!(nftables_output.nftables.len(), 8);

        // 验证 metainfo
        match &nftables_output.nftables[0] {
            NftablesEntry::Metainfo {
                version,
                release_name,
                json_schema_version,
            } => {
                assert_eq!(version, "1.1.3");
                assert_eq!(release_name, "Commodore Bullmoose #4");
                assert_eq!(*json_schema_version, 1);
            }
            _ => panic!("Expected Metainfo entry"),
        }

        // 验证 table
        match &nftables_output.nftables[1] {
            NftablesEntry::Table {
                family,
                name,
                handle,
            } => {
                assert_eq!(family, "inet");
                assert_eq!(name, "filter");
                assert_eq!(*handle, 1);
            }
            _ => panic!("Expected Table entry"),
        }

        // 验证 chain
        match &nftables_output.nftables[2] {
            NftablesEntry::Chain {
                family,
                table,
                handle,
                name,
                r#type,
                hook,
                prio,
                policy,
            } => {
                assert_eq!(family, "inet");
                assert_eq!(table, "filter");
                assert_eq!(name, "input");
                assert_eq!(*handle, 1);
                assert_eq!(*r#type, Some("filter".to_string()));
                assert_eq!(*hook, Some("input".to_string()));
                assert_eq!(*prio, Some(0));
                assert_eq!(*policy, Some("accept".to_string()));
            }
            _ => panic!("Expected Chain entry"),
        }

        // 验证 set
        match &nftables_output.nftables[6] {
            NftablesEntry::Set {
                family,
                table,
                name,
                handle,
                r#type,
                policy: _,
                flags,
                elem: _,
            } => {
                assert_eq!(family, "ip");
                assert_eq!(name, "nb0000001");
                assert_eq!(table, "netbird");
                assert_eq!(*handle, 40);
                assert_eq!(*r#type, Some("ipv4_addr".to_string()));
                assert_eq!(*flags, Some(vec!["dynamic".to_string()]));
            }
            _ => panic!("Expected Set entry"),
        }

        // 验证 rule
        match &nftables_output.nftables[7] {
            NftablesEntry::Rule {
                family,
                table,
                chain,
                handle,
                expr,
            } => {
                assert_eq!(family, "ip");
                assert_eq!(table, "netbird");
                assert_eq!(chain, "netbird-rt-fwd");
                assert_eq!(*handle, 22);
                assert_eq!(expr.len(), 3);
            }
            _ => panic!("Expected Rule entry"),
        }
    }

    #[test]
    fn test_deserialize_unknown_entry() {
        let json_data = r#"{
    "nftables": [
        {
            "unknown_type": {
                "some_field": "some_value",
                "another_field": 123
            }
        }
    ]
}"#;

        let result: Result<NftablesOutput, _> = serde_json::from_str(json_data);
        assert!(
            result.is_ok(),
            "Failed to deserialize JSON with unknown entry: {:?}",
            result.err()
        );

        let nftables_output = result.unwrap();
        assert_eq!(nftables_output.nftables.len(), 1);

        // 验证未知类型被正确处理为 Unknown 变体
        match &nftables_output.nftables[0] {
            NftablesEntry::Unknown(value) => {
                assert!(value.is_object());
                let obj = value.as_object().unwrap();
                assert!(obj.contains_key("unknown_type"));
            }
            _ => panic!("Expected Unknown entry"),
        }
    }
}
