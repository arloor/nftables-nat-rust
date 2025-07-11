use std::io;
use std::net::{ToSocketAddrs, IpAddr};
use std::ops::Add;

// 专门用于解析IPv6地址的函数
pub fn remote_ipv6(domain: &String) -> io::Result<String> {
    // 首先尝试直接解析为IPv6地址
    if let Ok(ip) = domain.parse::<IpAddr>() {
        if ip.is_ipv6() {
            return Ok(ip.to_string());
        } else {
            return Err(io::Error::other("Domain resolved to IPv4 but IPv6 was requested"));
        }
    }

    // 如果不是IP地址，则进行DNS解析，只返回IPv6
    domain
        .to_string()
        .add(":80")
        .to_socket_addrs()?
        .find(|addr| addr.is_ipv6())
        .map(|addr| addr.ip().to_string())
        .ok_or_else(|| io::Error::other("Failed to resolve IPv6 address"))
}

// 专门用于解析IPv4地址的函数
pub fn remote_ipv4(domain: &String) -> io::Result<String> {
    // 首先尝试直接解析为IPv4地址
    if let Ok(ip) = domain.parse::<IpAddr>() {
        if ip.is_ipv4() {
            return Ok(ip.to_string());
        } else {
            return Err(io::Error::other("Domain resolved to IPv6 but IPv4 was requested"));
        }
    }

    // 如果不是IP地址，则进行DNS解析，只返回IPv4
    domain
        .to_string()
        .add(":80")
        .to_socket_addrs()?
        .find(|addr| addr.is_ipv4())
        .map(|addr| addr.ip().to_string())
        .ok_or_else(|| io::Error::other("Failed to resolve IPv4 address"))
}

#[allow(clippy::unwrap_used)]
mod test {

    // #[test]
    // fn test_default_src_ip() {
    //     use std::net::Ipv4Addr;
    //     let ip = super::default_src_ip().unwrap();
    //     println!("Default source IP: {}", ip);
    //     assert!(!ip.is_empty());
    //     assert!(ip.parse::<Ipv4Addr>().is_ok());
    // }
    #[test]
    fn test_remote_ipv4() {
        use std::net::Ipv4Addr;
        let domain = "www.google.com".to_string();
        let ip = super::remote_ipv4(&domain).unwrap();
        println!("Resolved IPv4 for {domain}: {ip}");
        assert!(!ip.is_empty());
        assert!(ip.parse::<Ipv4Addr>().is_ok());
    }

    #[test]
    fn test_remote_ipv4_fail() {
        let domain = "example.asddddddddddddddddddddaasdasdasdasdasdasadasads.com".to_string();
        let res = super::remote_ipv4(&domain);
        println!("Resolved IPv4 for {domain}: {res:?}");
        assert!(res.is_err());
    }
}
