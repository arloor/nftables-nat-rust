use std::io;
use std::net::ToSocketAddrs;
use std::ops::Add;

pub fn remote_ip(domain: &String) -> io::Result<String> {
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
    fn test_remote_ip() {
        use std::net::Ipv4Addr;
        let domain = "www.google.com".to_string();
        let ip = super::remote_ip(&domain).unwrap();
        println!("Resolved IP for {domain}: {ip}");
        assert!(!ip.is_empty());
        assert!(ip.parse::<Ipv4Addr>().is_ok());
    }

    #[test]
    fn test_remote_ip2() {
        let domain = "example.asddddddddddddddddddddaasdasdasdasdasdasadasads.com".to_string();
        let res = super::remote_ip(&domain);
        println!("Resolved IP for {domain}: {res:?}");
        assert!(res.is_err());
    }
}
