mod IP;
mod config;

use std::collections::HashMap;
use std::process::{exit, Command};
use crate::IP::remote_ip;
use std::fs::File;
use std::io::Write;

fn main() {
    //脚本的前缀
    let script_prefix=String::from("#!/usr/sbin/nft -f\n\
    \n\
    flush ruleset\n\
    add table ip nat\n\
    add chain nat PREROUTING { type nat hook prerouting priority -100 ; }\n\
    add chain nat POSTROUTING { type nat hook postrouting priority 100 ; }\n");

    let vec = config::read_config();
    let mut script = String::new();
    script += &script_prefix;

    for x in vec.iter() {
        let (domain, ip) = x.get_target_ip();
        println!("{}-{}", domain, ip);
        let string = x.build();
        script += &string;
//        println!("{}",string)
    }
    println!("{}", script);

    let mut f=File::create("temp.nft");
    if let Ok(mut file) = f{
        {println!("{:?}",file)}
        file.write_all(script.as_bytes()).expect("写失败");
    }
    let output = Command::new("/usr/sbin/nft")
        .arg("-f")
        .arg("temp.nft")
        .output()
        .unwrap_or_else(|e| panic!("wg panic because:{}", e));
    println!("output:");
    let st = String::from_utf8_lossy(&output.stdout);
    let lines = st.split("\n");
    for line in lines {
        println!("{}", line);
    }
}