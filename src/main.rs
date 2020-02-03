mod local_ip;

use std::collections::HashMap;
use std::process::exit;

fn main() {
    let (mut port_mark, local_ip) = init();
    println!("{}", local_ip);
    read_old();
}

#[derive(Debug)]
enum nat_cell {
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
    fn getType(&self) -> nat_cell {
        match &self {
            nat_cell::SINGLE { local_port, remote_port, remote_domain } => {
                nat_cell::SINGLE {
                    local_port: 0,
                    remote_port: 0,
                    remote_domain: "".to_string(),
                }
            }
            _ =>nat_cell::RANGE {
                port_start: 0,
                port_end: 0,
                remote_domain: "".to_string()
            }
        }
    }
}

// 生成一个长度为65536的数组，用于标记端口占用情况
// 返回本地IP
fn init() -> ([i8; 65536], String) {
    let local_ip = local_ip::get();
    let local_ip = match local_ip {
        Some(val) => {
            val
        }
        None => {
            println!("不能获取本地IP。退出程序");
            exit(-1);
        }
    };
    ([0; 65536], local_ip)
}


//读取旧的配置
fn read_old() {
    let mut nat_cells = vec![];
    nat_cells.push(nat_cell::RANGE {
        port_start: 1,
        port_end: 100,
        remote_domain: String::from("arloor.com"),
    });

    let cell = nat_cells.get(0).expect("aaaa");

    println!("{:#?}", cell);
     match cell {
        nat_cell::SINGLE { local_port, remote_port, remote_domain } => {
            println!("single");
        }

        _ => {}
    }
}