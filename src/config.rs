// SINGLE,8100,8100,arloor.com
// RANGE,1000,2000,arloor.com
use std::fs;
use std::fs::File;
use crate::IP::{remote_ip, local_ip};

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
        match &self {
            nat_cell::RANGE { port_start, port_end, remote_domain } =>
                build_range(port_start, port_end, remote_domain)
            ,
            nat_cell::SINGLE { local_port, remote_port, remote_domain } =>
                build_single(local_port, remote_port, remote_domain)
        }
    }

    pub fn get_target_ip(&self) -> (String, String) {
        match &self {
            nat_cell::RANGE { port_start, port_end, remote_domain } =>
                (remote_domain.parse().unwrap(), match remote_ip(remote_domain) {
                    Some(s) => s,
                    None => "".to_string()
                })
            ,
            nat_cell::SINGLE { local_port, remote_port, remote_domain } =>
                (remote_domain.parse().unwrap(), match remote_ip(remote_domain) {
                    Some(s) => s,
                    None => "".to_string()
                })
        }
    }
}

fn build_single(local_port: &i32, remote_port: &i32, remote_domain: &String) -> String {
    let string = format!("add rule ip nat PREROUTING tcp dport {localPort} counter dnat to {remoteIP}:{remotePort}\n\
    add rule ip nat PREROUTING udp dport {localPort} counter dnat to {remoteIP}:{remotePort}\n\
    add rule ip nat POSTROUTING ip daddr {remoteIP} tcp dport {remotePort} counter snat to {localIP}\n\
    add rule ip nat POSTROUTING ip daddr {remoteIP} udp dport {remotePort} counter snat to {localIP}\n\
    ", localPort = local_port, remotePort = remote_port, remoteIP = match remote_ip(remote_domain) {
        Some(s) => s,
        None => return "".to_string(),
    }, localIP = match local_ip() {
        Some(s) => s,
        None => return "".to_string(),
    });
    string
}


fn build_range(port_start: &i32, port_end: &i32, remote_domain: &String) -> String {
    let string = format!("add rule ip nat PREROUTING tcp dport {portStart}-{portEnd} counter dnat to {remoteIP}:{portStart}-{portEnd}\n\
    add rule ip nat PREROUTING udp dport {portStart}-{portEnd} counter dnat to {remoteIP}:{portStart}-{portEnd}\n\
    add rule ip nat POSTROUTING ip daddr {remoteIP} tcp dport {portStart}-{portEnd} counter snat to {localIP}\n\
    add rule ip nat POSTROUTING ip daddr {remoteIP} udp dport {portStart}-{portEnd} counter snat to {localIP}\n\
    ", portStart = port_start, portEnd = port_end, remoteIP = match remote_ip(remote_domain) {
        Some(s) => s,
        None => return "".to_string(),
    }, localIP = match local_ip() {
        Some(s) => s,
        None => return "".to_string(),
    });

    string
}


pub fn read_config() -> Vec<nat_cell> {
    let mut nat_cells = vec![];


    let mut contents = fs::read_to_string("nat.config")
        .expect("Something went wrong reading the file");
    contents = contents.replace("\r\n", "\n");

    let strs = contents.split("\n");
    for str in strs {
        let cells = str.split(",").collect::<Vec<&str>>();
        if cells.len() == 4 {
            if cells[0] == "RANGE" {
                nat_cells.push(nat_cell::RANGE {
                    port_start: cells[1].parse::<i32>().unwrap(),
                    port_end: cells[2].parse::<i32>().unwrap(),
                    remote_domain: String::from(cells[3]),
                });
            }
            if cells[0] == "SINGLE" {
                nat_cells.push(nat_cell::SINGLE {
                    local_port: cells[1].parse::<i32>().unwrap(),
                    remote_port: cells[2].parse::<i32>().unwrap(),
                    remote_domain: String::from(cells[3]),
                });
            }
        } else {
            println!("{} is not valid", str)
        }
    }
    nat_cells
}