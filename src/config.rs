// SINGLE,8100,8100,arloor.com
// RANGE,1000,2000,arloor.com
use std::fs::{self, File};
use crate::ip;
use std::process::exit;

#[derive(Debug)]
pub enum nat_cell {
    SINGLE {
        local_port: i32,
        remote_port: i32,
        remote_domain: String,
    },
    RANGE {
        port_start: i32,
        port_end: i32,
        remote_domain: String,
    },
}

impl nat_cell {
    pub fn build(&self) -> String {
        let remote_domain = match &self {
            nat_cell::SINGLE { remote_domain, .. } => remote_domain,
            nat_cell::RANGE { remote_domain, .. } => remote_domain
        };
        let remoteIP = match ip::remote_ip(remote_domain) {
            Some(s) => s,
            None => return "".to_string(),
        };
        let localIP = match ip::local_ip() {
            Some(s) => s,
            None => return "".to_string(),
        };
        match &self {
            nat_cell::RANGE { port_start, port_end, remote_domain } =>
                {
                    format!("#{cell:?}\n\
                    add rule ip nat PREROUTING tcp dport {portStart}-{portEnd} counter dnat to {remoteIP}:{portStart}-{portEnd}\n\
                    add rule ip nat PREROUTING udp dport {portStart}-{portEnd} counter dnat to {remoteIP}:{portStart}-{portEnd}\n\
                    add rule ip nat POSTROUTING ip daddr {remoteIP} tcp dport {portStart}-{portEnd} counter snat to {localIP}\n\
                    add rule ip nat POSTROUTING ip daddr {remoteIP} udp dport {portStart}-{portEnd} counter snat to {localIP}\n\n\
                    ", cell = self, portStart = port_start, portEnd = port_end, remoteIP = remoteIP, localIP = localIP)
                }
            nat_cell::SINGLE { local_port, remote_port, remote_domain } =>
                {
                    format!("#{cell:?}\n\
                    add rule ip nat PREROUTING tcp dport {localPort} counter dnat to {remoteIP}:{remotePort}\n\
                    add rule ip nat PREROUTING udp dport {localPort} counter dnat to {remoteIP}:{remotePort}\n\
                    add rule ip nat POSTROUTING ip daddr {remoteIP} tcp dport {remotePort} counter snat to {localIP}\n\
                    add rule ip nat POSTROUTING ip daddr {remoteIP} udp dport {remotePort} counter snat to {localIP}\n\n\
                    ", cell = self, localPort = local_port, remotePort = remote_port, remoteIP = remoteIP, localIP = localIP)
                }
        }
    }

    pub fn get_target_ip(&self) -> (String, String) {
        match &self {
            nat_cell::RANGE { port_start, port_end, remote_domain } =>
                (remote_domain.parse().unwrap(), match ip::remote_ip(remote_domain) {
                    Some(s) => s,
                    None => "".to_string()
                })
            ,
            nat_cell::SINGLE { local_port, remote_port, remote_domain } =>
                (remote_domain.parse().unwrap(), match ip::remote_ip(remote_domain) {
                    Some(s) => s,
                    None => "".to_string()
                })
        }
    }
}


pub fn example(conf: &String) {
    println!("请在 {} 编写转发规则，内容类似：", &conf);
    println!("{}", "SINGLE,10000,443,baidu.com\n\
                    RANGE,1000,2000,baidu.com")
}

pub fn read_config(conf: String) -> Vec<nat_cell> {
    let mut nat_cells = vec![];
    let mut contents = match fs::read_to_string(&conf) {
        Ok(s) => s,
        Err(e) => {
            example(&conf);
            exit(1);
        }
    };
    contents = contents.replace("\r\n", "\n");

    let strs = contents.split("\n");
    for str in strs {
        let cells = str.trim().split(",").collect::<Vec<&str>>();
        if cells.len() == 4 {
            if cells[0].trim() == "RANGE" {
                nat_cells.push(nat_cell::RANGE {
                    port_start: cells[1].trim().parse::<i32>().unwrap(),
                    port_end: cells[2].trim().parse::<i32>().unwrap(),
                    remote_domain: String::from(cells[3].trim()),
                });
            }
            if cells[0].trim() == "SINGLE" {
                nat_cells.push(nat_cell::SINGLE {
                    local_port: cells[1].trim().parse::<i32>().unwrap(),
                    remote_port: cells[2].trim().parse::<i32>().unwrap(),
                    remote_domain: String::from(cells[3].trim()),
                });
            }
        } else if str.trim().len() != 0 {
            println!("#! {} is not valid", str)
        }
    }
    nat_cells
}