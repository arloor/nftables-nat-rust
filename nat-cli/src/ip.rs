use std::io;
use std::net::{ToSocketAddrs, IpAddr};
use std::ops::Add;
use crate::config::IpVersion;

// 统一的IP地址解析函数，支持IPv4、IPv6和Both模式
pub fn remote_ip(domain: &String, ip_version: &IpVersion) -> io::Result<String> {
    // 首先尝试直接解析为IP地址
    if let Ok(ip) = domain.parse::<IpAddr>() {
        match ip_version {
            IpVersion::V4 => {
                if ip.is_ipv4() {
                    return Ok(ip.to_string());
                } else {
                    return Err(io::Error::other("Domain resolved to IPv6 but IPv4 was requested"));
                }
            }
            IpVersion::V6 => {
                if ip.is_ipv6() {
                    return Ok(ip.to_string());
                } else {
                    return Err(io::Error::other("Domain resolved to IPv4 but IPv6 was requested"));
                }
            }
            IpVersion::Both => {
                return Ok(ip.to_string());
            }
        }
    }

    // 如果不是IP地址，则进行DNS解析
    let socket_addrs: Vec<_> = domain
        .to_string()
        .add(":80")
        .to_socket_addrs()?
        .collect();

    match ip_version {
        IpVersion::V4 => {
            socket_addrs
                .iter()
                .find(|addr| addr.is_ipv4())
                .map(|addr| addr.ip().to_string())
                .ok_or_else(|| io::Error::other("Failed to resolve IPv4 address"))
        }
        IpVersion::V6 => {
            socket_addrs
                .iter()
                .find(|addr| addr.is_ipv6())
                .map(|addr| addr.ip().to_string())
                .ok_or_else(|| io::Error::other("Failed to resolve IPv6 address"))
        }
        IpVersion::Both => {
            // 优先IPv4，如果没有IPv4则使用IPv6
            socket_addrs
                .iter()
                .find(|addr| addr.is_ipv4())
                .or_else(|| socket_addrs.iter().find(|addr| addr.is_ipv6()))
                .map(|addr| addr.ip().to_string())
                .ok_or_else(|| io::Error::other("Failed to resolve any IP address"))
        }
    }
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
    fn test_remote_ip_v4() {
        use std::net::Ipv4Addr;
        use super::IpVersion;
        let domain = "www.google.com".to_string();
        let ip = super::remote_ip(&domain, &IpVersion::V4).unwrap();
        println!("Resolved IPv4 for {domain}: {ip}");
        assert!(!ip.is_empty());
        assert!(ip.parse::<Ipv4Addr>().is_ok());
    }

    #[test]
    fn test_remote_ip_both() {
        use super::IpVersion;
        let domain = "www.google.com".to_string();
        let ip = super::remote_ip(&domain, &IpVersion::Both).unwrap();
        println!("Resolved IP (Both mode) for {domain}: {ip}");
        assert!(!ip.is_empty());
        // Should resolve to either IPv4 or IPv6, but prefer IPv4
        assert!(ip.parse::<std::net::IpAddr>().is_ok());
    }

    #[test]
    fn test_resolve_localhost() {
        use super::IpVersion;
        let domain = "localhost".to_string();
        let ip = super::remote_ip(&domain, &IpVersion::Both).unwrap();
        println!("Resolved IP (Both mode) for {domain}: {ip}");
        assert!(!ip.is_empty());
        // Should resolve to either IPv4 or IPv6, but prefer IPv4
        assert!(ip.parse::<std::net::IpAddr>().is_ok());
        
        let ip = super::remote_ip(&domain, &IpVersion::V6).unwrap();
        println!("Resolved IP (V6) for {domain}: {ip}");
        assert!(!ip.is_empty());
        // Should resolve to either IPv4 or IPv6, but prefer IPv4
        assert!(ip.parse::<std::net::IpAddr>().is_ok());
    }

    #[test]
    fn test_remote_ip_fail() {
        use super::IpVersion;
        let domain = "example.asddddddddddddddddddddaasdasdasdasdasdasadasads.com".to_string();
        let res = super::remote_ip(&domain, &IpVersion::V4);
        println!("Resolved IPv4 for {domain}: {res:?}");
        assert!(res.is_err());
    }

}
