#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
mod config;
mod ip;
mod logger;
mod prepare;

use clap::Parser;
use log::info;
use std::fs::File;
use std::io::{self, Write};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const NFTABLES_ETC: &str = "/etc/nftables-nat";
const FILE_NAME_SCRIPT: &str = "/etc/nftables-nat/nat-diy.nft";
const IP_FORWARD: &str = "/proc/sys/net/ipv4/ip_forward";
const CARGO_CRATE_NAME: &str = env!("CARGO_CRATE_NAME");

/// A nftables NAT management tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 配置文件路径
    #[arg(value_name = "CONFIG_FILE", help = "老版本配置文件")]
    compatible_config_file: Option<String>,
    #[arg(long, value_name = "TOML_CONFIG", help = "toml配置文件")]
    toml: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init(CARGO_CRATE_NAME);
    // 使用 clap 解析命令行参数
    let args = Args::parse();
    let nat_cells = parse_conf(args)?;
    info!("读取配置文件成功: ");
    for ele in &nat_cells {
        info!("{:?}", ele);
    }

    global_prepare()?;
    Ok(handle_loop(nat_cells)?)
}

fn parse_conf(args: Args) -> Result<Vec<config::NatCell>, Box<dyn std::error::Error>> {
    let nat_cells = if let Some(compatible_config_file) = args.compatible_config_file {
        info!("使用老版本配置文件: {:?}", compatible_config_file);
        config::read_config(&compatible_config_file).map_err(|e| {
            info!("读取配置文件失败: {:?}", e);
            config::example(&compatible_config_file);
            e
        })?
    } else if let Some(toml) = args.toml {
        info!("使用toml配置文件: {:?}", toml);
        config::read_toml_config(&toml).map_err(|e| {
            info!("读取配置文件失败: {:?}", e);
            if let Err(e) = config::toml_example(&toml) {
                info!("{:?}", e);
            }
            e
        })?
    } else {
        return Err("请提供配置文件路径".into());
    };
    Ok(nat_cells)
}

fn global_prepare() -> Result<(), io::Error> {
    let _ = std::fs::create_dir_all(NFTABLES_ETC);
    // 修改内核参数，开启端口转发
    match std::fs::write(IP_FORWARD, "1") {
        Ok(_s) => {
            info!("kernel ip_forward config enabled!\n")
        }
        Err(e) => {
            info!("enable ip_forward FAILED! cause: {:?}\nPlease excute `echo 1 > /proc/sys/net/ipv4/ip_forward` manually\n", e);
            return Err(e);
        }
    };
    Ok(())
}

fn handle_loop(nat_cells: Vec<config::NatCell>) -> Result<(), io::Error> {
    let mut latest_script = String::new();
    loop {
        let script = build_new_script(&nat_cells)?;
        prepare::check_and_prepare()?;
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
                "执行/usr/sbin/nft -f {FILE_NAME_SCRIPT} 执行结果: {}",
                output.status
            );
            log::info!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            log::error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            info!("WAIT:等待配置或目标IP发生改变....\n");
        }

        //等待60秒
        sleep(Duration::new(60, 0));
    }
}

fn build_new_script(nat_cells: &[config::NatCell]) -> Result<String, io::Error> {
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

    let mut script = String::new();
    script += &script_prefix;

    for x in nat_cells.iter() {
        script += &x.build()?;
    }
    Ok(script)
}
