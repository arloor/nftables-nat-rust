use std::io;
use std::net::ToSocketAddrs;
use std::net::UdpSocket;
use std::ops::Add;
use std::sync::LazyLock;

pub(crate) static LOCAL_IP: LazyLock<io::Result<String>> = LazyLock::new(default_src_ip);

fn default_src_ip() -> io::Result<String> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    socket
        .local_addr()
        .map(|local_addr| local_addr.ip().to_string())
}

pub fn remote_ip(domain: &String) -> io::Result<String> {
    domain
        .to_string()
        .add(":80")
        .to_socket_addrs()?
        .find(|addr| addr.is_ipv4())
        .map(|addr| addr.ip().to_string())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to resolve IPv4 address"))
}
