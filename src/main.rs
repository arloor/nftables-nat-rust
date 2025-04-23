#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
mod config;
mod ip;
mod logger;
mod prepare;

use log::info;
use std::env;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const NFTABLES_ETC: &str = "/etc/nftables-nat";
const FILE_NAME_SCRIPT: &str = "/etc/nftables-nat/nat-diy.nft";
const IP_FORWARD: &str = "/proc/sys/net/ipv4/ip_forward";
const CARGO_CRATE_NAME: &str = env!("CARGO_CRATE_NAME");
fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init(CARGO_CRATE_NAME);
    let args: Vec<String> = env::args().collect();
    let mut conf = String::new();
    if args.len() != 2 {
        let conf = "nat.conf".to_string();
        info!("{}{}", "使用方式：nat ", conf);
        config::example(&conf);
        return Ok(());
    } else {
        conf += &args[1];
    }

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

    let mut latest_script = String::new();

    loop {
        prepare::check_and_prepare()?;
        let script = build_new_script(&conf);
        if script != latest_script {
            info!("nftables脚本如下：\n{}", script);
            latest_script.clone_from(&script);
            let f = File::create(FILE_NAME_SCRIPT);
            if let Ok(mut file) = f {
                file.write_all(script.as_bytes())?;
            }

            let output = Command::new("/usr/sbin/nft")
                .arg("-f")
                .arg(FILE_NAME_SCRIPT)
                .output()?;
            info!(
                "执行/usr/sbin/nft -f {FILE_NAME_SCRIPT}\n执行结果: {}",
                output.status
            );
            // io::stdout().write_all(&output.stdout)?;
            // io::stderr().write_all(&output.stderr)?;
            info!("WAIT:等待配置或目标IP发生改变....\n");
        }

        //等待60秒
        sleep(Duration::new(60, 0));
    }
}

fn build_new_script(conf: &str) -> String {
    //脚本的前缀
    let script_prefix = String::from(
        "#!/usr/sbin/nft -f\n\
        \n\
        add table ip self-nat\n\
        delete table ip self-nat\n\
        add table ip self-nat\n\
        add chain self-nat PREROUTING { type nat hook prerouting priority -110 ; }\n\
        add chain self-nat POSTROUTING { type nat hook postrouting priority 110 ; }\n\
        ",
    );

    let vec = config::read_config(conf);
    let mut script = String::new();
    script += &script_prefix;

    for x in vec.iter() {
        match x.build() {
            Ok(string) => {
                script += &string;
            }
            Err(e) => {
                info!("build error: {:?}", e);
            }
        }
    }
    script
}
