#![deny(warnings)]
mod config;
mod ip;

use log::info;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::{env, io};

const NFTABLES_ETC: &str = "/etc/nftables";
const IP_FORWARD: &str = "/proc/sys/net/ipv4/ip_forward";

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

// 检查 nftables 表、链和规则是否存在
fn check_nftables_entities() -> (bool, bool, bool, bool, bool, bool) {
    let output = Command::new("/usr/sbin/nft")
        .arg("-j")
        .arg("list")
        .arg("ruleset")
        .output()
        .expect("执行 nft -j list ruleset 失败");

    if !output.status.success() {
        info!("执行 nft -j list ruleset 命令失败");
        return (false, false, false, false, false, false);
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let nftables_output: NftablesOutput = match serde_json::from_str(&json_str) {
        Ok(output) => output,
        Err(e) => {
            info!("解析 nft 输出的 JSON 失败: {}", e);
            return (false, false, false, false, false, false);
        }
    };

    let mut nat_table_exists = false;
    let mut prerouting_chain_exists = false;
    let mut postrouting_chain_exists = false;
    let mut diy_prerouting_chain_exists = false;
    let mut diy_postrouting_chain_exists = false;
    let mut jump_rule_count = 0;

    for entry in nftables_output.nftables {
        match entry {
            NftablesEntry::Table { table } => {
                if table.family == "ip" && table.name == "nat" {
                    nat_table_exists = true;
                }
            }
            NftablesEntry::Chain { chain } => {
                if chain.family == "ip" && chain.table == "nat" {
                    match chain.name.as_str() {
                        "PREROUTING" => prerouting_chain_exists = true,
                        "POSTROUTING" => postrouting_chain_exists = true,
                        "DIY_PREROUTING" => diy_prerouting_chain_exists = true,
                        "DIY_POSTROUTING" => diy_postrouting_chain_exists = true,
                        _ => {}
                    }
                }
            }
            NftablesEntry::Rule { rule } => {
                if rule.family == "ip"
                    && rule.table == "nat"
                    && (rule.chain == "PREROUTING" || rule.chain == "POSTROUTING")
                {
                    // 检查是否有跳转到DIY_*链的规则
                    let is_jump_rule = rule.expr.iter().any(|expr| {
                        if let Some(jump) = expr.as_object().and_then(|obj| obj.get("jump")) {
                            if let Some(target) = jump.as_object().and_then(|obj| obj.get("target"))
                            {
                                let target_str = target.as_str().unwrap_or("");
                                return (rule.chain == "PREROUTING"
                                    && target_str == "DIY_PREROUTING")
                                    || (rule.chain == "POSTROUTING"
                                        && target_str == "DIY_POSTROUTING");
                            }
                        }
                        false
                    });

                    if is_jump_rule {
                        jump_rule_count += 1;
                    }
                }
            }
            _ => {}
        }
    }

    // 如果至少有一个跳转规则，我们认为跳转规则存在
    let jump_rules_exist = jump_rule_count > 0;

    info!("nat表存在: {}", nat_table_exists);
    info!("PREROUTING链存在: {}", prerouting_chain_exists);
    info!("POSTROUTING链存在: {}", postrouting_chain_exists);
    info!("DIY_PREROUTING链存在: {}", diy_prerouting_chain_exists);
    info!("DIY_POSTROUTING链存在: {}", diy_postrouting_chain_exists);
    info!("跳转规则存在: {}", jump_rules_exist);

    (
        nat_table_exists,
        prerouting_chain_exists,
        postrouting_chain_exists,
        diy_prerouting_chain_exists,
        diy_postrouting_chain_exists,
        jump_rules_exist,
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log_x::init_log("log", "nat.log")?;

    let _ = std::fs::create_dir_all(NFTABLES_ETC);
    // 修改内核参数，开启端口转发
    match std::fs::write(IP_FORWARD, "1") {
        Ok(_s) => {
            info!("kernel ip_forward config enabled!\n")
        }
        Err(e) => {
            info!("enable ip_forward FAILED! cause: {:?}\nPlease excute `echo 1 > /proc/sys/net/ipv4/ip_forward` manually\n", e)
        }
    };

    let args: Vec<String> = env::args().collect();
    let mut latest_script = String::new();

    loop {
        let mut conf = String::new();
        if args.len() != 2 {
            let conf = "nat.conf".to_string();
            info!("{}{}", "使用方式：nat ", conf);
            config::example(&conf);
            return Ok(());
        } else {
            conf += &args[1];
        }

        // 检查当前 nftables 中表、链和规则的存在情况
        let (
            nat_table_exists,
            prerouting_chain_exists,
            postrouting_chain_exists,
            diy_prerouting_chain_exists,
            diy_postrouting_chain_exists,
            jump_rules_exist,
        ) = check_nftables_entities();

        // 根据检查结果动态构建脚本前缀
        let mut script_prefix = String::from("#!/usr/sbin/nft -f\n\n");

        // 只有在相关实体不存在时才添加相应的命令
        if !nat_table_exists {
            script_prefix.push_str("# 确保 nat 表存在（不删除整个表）\n");
            script_prefix.push_str("add table ip nat\n");
        }

        if !prerouting_chain_exists {
            script_prefix.push_str("# 确保 PREROUTING 链存在\n");
            script_prefix.push_str(
                "add chain nat PREROUTING { type nat hook prerouting priority -100 ; }\n",
            );
        }

        if !postrouting_chain_exists {
            script_prefix.push_str("# 确保 POSTROUTING 链存在\n");
            script_prefix.push_str(
                "add chain nat POSTROUTING { type nat hook postrouting priority 100 ; }\n",
            );
        }

        if !diy_prerouting_chain_exists {
            script_prefix.push_str("# 创建自定义 DIY_PREROUTING 链\n");
            script_prefix.push_str("add chain ip nat DIY_PREROUTING{}\n");
        }

        if !diy_postrouting_chain_exists {
            script_prefix.push_str("# 创建自定义 DIY_POSTROUTING 链\n");
            script_prefix.push_str("add chain ip nat DIY_POSTROUTING{}\n");
        }

        if !jump_rules_exist {
            script_prefix.push_str("# 在预定义链中添加跳转规则\n");
            script_prefix.push_str("add rule ip nat PREROUTING jump DIY_PREROUTING\n");
            script_prefix.push_str("add rule ip nat POSTROUTING jump DIY_POSTROUTING\n");
        }

        script_prefix.push('\n');
        script_prefix.push_str("# 清空自定义链中的规则\n");
        script_prefix.push_str("flush chain ip nat DIY_PREROUTING\n");
        script_prefix.push_str("flush chain ip nat DIY_POSTROUTING\n");
        script_prefix.push('\n');

        let vec = config::read_config(conf);
        let mut script = String::new();
        script += &script_prefix;

        for x in vec.iter() {
            let string = x.build();
            script += &string;
        }

        //如果是linux，且生成的脚本产生变化，则写到文件，并且执行
        if script != latest_script {
            info!("nftables脚本如下：\n{}", script);
            latest_script.clone_from(&script);
            if cfg!(target_os = "linux") {
                let f = File::create("/etc/nftables/nat-diy.nft");
                if let Ok(mut file) = f {
                    file.write_all(script.as_bytes()).expect("写失败");
                }

                let output = Command::new("/usr/sbin/nft")
                    .arg("-f")
                    .arg("/etc/nftables/nat-diy.nft")
                    .output()
                    .expect("/usr/sbin/nft invoke error");
                info!(
                    "执行/usr/sbin/nft -f /etc/nftables/nat-diy.nft\n执行结果: {}",
                    output.status
                );
                io::stdout()
                    .write_all(&output.stdout)
                    .unwrap_or_else(|e| info!("error {}", e));
                io::stderr()
                    .write_all(&output.stderr)
                    .unwrap_or_else(|e| info!("error {}", e));
                info!("WAIT:等待配置或目标IP发生改变....\n");
            }
        }

        //等待60秒
        sleep(Duration::new(60, 0));
    }
}
