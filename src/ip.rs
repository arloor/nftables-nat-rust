use std::io;
use std::net::UdpSocket;
use std::ops::Add;

pub fn local_ip() -> io::Result<String> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    socket.local_addr().map(|local_addr| local_addr.ip().to_string())
}

pub fn remote_ip(domain: &String) -> io::Result<String> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(domain.to_string().add(":80"))?;
    socket.peer_addr().map(|addr| addr.ip().to_string())
}