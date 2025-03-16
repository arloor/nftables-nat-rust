#![deny(warnings)]
mod config;
mod ip;

use crate::config::NatCell;
use log::info;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::{env, io};

const NFTABLES_ETC: &str = "/etc/nftables";
const IP_FORWARD: &str = "/proc/sys/net/ipv4/ip_forward";

fn main() -> Result<(), Box<dyn std::error::Error>>{
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

        //脚本的前缀
        let script_prefix = String::from(
            "#!/usr/sbin/nft -f\n\
        \n\
        add table ip nat\n\
        delete table ip nat\n\
        add table ip nat\n\
        add chain nat PREROUTING { type nat hook prerouting priority -100 ; }\n\
        add chain nat POSTROUTING { type nat hook postrouting priority 100 ; }\n\n",
        );

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
                // 1. 首先将生成的规则写入文件
                let f = File::create("/etc/nftables/nat-diy.nft");
                if let Ok(mut file) = f {
                    file.write_all(script.as_bytes()).expect("写失败");
                }

                // 2. 先刷新所有规则
                let _ = Command::new("/usr/sbin/nft")
                    .arg("flush")
                    .arg("ruleset")
                    .output();

                // 3. 应用系统默认配置
                let _ = Command::new("/usr/sbin/nft")
                    .arg("-f")
                    .arg("/etc/nftables.conf")
                    .output();

                // 4. 再应用我们的NAT规则
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

                // 获取所有配置的目标IP并执行ip rule命令
                for x in vec.iter() {
                    if let Some(dst_domain) = match x {
                        NatCell::Single { dst_domain, .. } => Some(dst_domain),
                        NatCell::Range { dst_domain, .. } => Some(dst_domain),
                        NatCell::Comment { .. } => None,
                    } {
                        if let Ok(dst_ip) = ip::remote_ip(dst_domain) {
                            // 先删除可能存在的旧规则
                            let _ = Command::new("ip")
                                .arg("rule")
                                .arg("del")
                                .arg("from")
                                .arg(&dst_ip)
                                .arg("lookup")
                                .arg("CM")
                                .output();

                            // 添加新规则
                            let output = Command::new("ip")
                                .arg("rule")
                                .arg("add")
                                .arg("from")
                                .arg(&dst_ip)
                                .arg("lookup")
                                .arg("CM")
                                .output();

                            match output {
                                Ok(output) => {
                                    info!(
                                        "执行 ip rule add from {} lookup CM\n执行结果: {}",
                                        dst_ip, output.status
                                    );
                                    if !output.status.success() {
                                        info!("错误输出: {}", String::from_utf8_lossy(&output.stderr));
                                    }
                                }
                                Err(e) => info!("执行ip rule命令失败: {}", e),
                            }
                        }
                    }
                }

                info!("WAIT:等待配置或目标IP发生改变....\n");
            }
        }

        //等待60秒
        sleep(Duration::new(60, 0));
    }
}
