#![deny(warnings)]
mod config;
mod ip;

use log::info;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::{env, io};

const NFTABLES_ETC: &str = "/etc/nftables";
const IP_FORWARD: &str = "/proc/sys/net/ipv4/ip_forward";

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

        //脚本的前缀
        let script_prefix = String::from(
            "#!/usr/sbin/nft -f\n\
        \n\
        # 确保 nat 表存在（不删除整个表）\n\
        add table ip nat\n\
        # 确保 PREROUTING和POSTROUTING链存在 \n\
        add chain nat PREROUTING { type nat hook prerouting priority -100 ; }\n\
        add chain nat POSTROUTING { type nat hook postrouting priority 100 ; }\n\
        # 创建我们的自定义链\n\
        add chain ip nat DIY_PREROUTING{}\n\
        add chain ip nat DIY_POSTROUTING{}\n\
        # 清空自定义链中上次的规则 \n\
        flush chain ip nat DIY_PREROUTING\n\
        flush chain ip nat DIY_POSTROUTING\n\
        # 在预定义链中添加跳转规则（如果不存在）todo 这里有重复\n\
        add rule ip nat PREROUTING jump DIY_PREROUTING \n\
        add rule ip nat POSTROUTING jump DIY_POSTROUTING \n\n",
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
