mod IP;
mod config;

use std::collections::HashMap;
use std::process::{exit, Command};
use crate::IP::remote_ip;
use std::fs::File;
use std::io::Write;
use std::{io, env};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

fn main() {
    std::fs::create_dir_all("/etc/nftables");
    match std::fs::write("/proc/sys/net/ipv4/ip_forward","1") {
        Ok(s)=>{println!("kernel ip_forward config enabled!\n")}
        Err(e)=>{println!("enable ip_forward FAILED! cause: {:?}\nPlease excute `echo 1 > /proc/sys/net/ipv4/ip_forward` manually\n",e)}
    };

    let args: Vec<String> = env::args().collect();
    let mut latest_script = String::new();

    loop {
        let mut conf = String::new();
        if args.len() != 2 {
            let conf = "nat.conf".to_string();
            println!("{}{}", "使用方式：nat ", conf);
            config::example(&conf);
            return;
        } else {
            conf += &args[1];
        }

        //脚本的前缀
        let script_prefix = String::from("#!/usr/sbin/nft -f\n\
    \n\
    add table ip nat\n\
    delete table ip nat\n\
    add table ip nat\n\
    add chain nat PREROUTING { type nat hook prerouting priority -100 ; }\n\
    add chain nat POSTROUTING { type nat hook postrouting priority 100 ; }\n\n");

        let vec = config::read_config(conf);
        let mut script = String::new();
        script += &script_prefix;

        for x in vec.iter() {
            let (domain, ip) = x.get_target_ip();
            let string = x.build();
            script += &string;
        }

        //如果是linux，且生成的脚本产生变化，则写到文件，并且执行
        if script != latest_script {
            println!("nftables脚本如下：\n{}", script);
            latest_script = script.clone();
            if cfg!(target_os = "linux") {
                let mut f = File::create("/etc/nftables/nat-diy.nft");
                if let Ok(mut file) = f {
                    file.write_all(script.as_bytes()).expect("写失败");
                }

                let output = Command::new("/usr/sbin/nft")
                    .arg("-f")
                    .arg("/etc/nftables/nat-diy.nft")
                    .output()
                    .unwrap_or_else(|e| panic!("wg panic because:{}", e));
                println!("执行/usr/sbin/nft -f /etc/nftables/nat-diy.nft\n执行结果: {}", output.status);
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();
                println!("WAIT:等待配置或目标IP发生改变....\n");
            }
        }

        //等待60秒
        sleep(Duration::new(60, 0));
    }
}