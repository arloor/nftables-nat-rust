use crate::config::RuntimeCell;
use crate::ip;
use ipnetwork::IpNetwork;
use log::warn;
use nat_common::{Chain, IpVersion, NftCell, Protocol, range_dnat_ports};
use std::env;
use std::io;
use std::net::IpAddr;
use std::str::FromStr;

const CT_MARK: &str = "0x4e4154";
const TABLES: [(&str, &str); 4] = [
    ("ip", "self-nat"),
    ("ip6", "self-nat"),
    ("ip", "self-filter"),
    ("ip6", "self-filter"),
];

#[derive(Clone, Copy)]
enum Family {
    Ip,
    Ip6,
}

impl Family {
    fn name(self) -> &'static str {
        match self {
            Family::Ip => "ip",
            Family::Ip6 => "ip6",
        }
    }

    fn addr_type(self) -> &'static str {
        match self {
            Family::Ip => "ipv4_addr",
            Family::Ip6 => "ipv6_addr",
        }
    }

    fn dnat_kw(self) -> &'static str {
        match self {
            Family::Ip => "ip",
            Family::Ip6 => "ip6",
        }
    }

    fn localhost(self) -> &'static str {
        match self {
            Family::Ip => "127.0.0.1",
            Family::Ip6 => "::1",
        }
    }

    fn snat_env(self) -> &'static str {
        match self {
            Family::Ip => "nat_local_ip",
            Family::Ip6 => "nat_local_ipv6",
        }
    }

    fn fmt_dnat_ip(self, ip: &str) -> String {
        match self {
            Family::Ip => ip.to_string(),
            Family::Ip6 => format!("[{ip}]"),
        }
    }
}

#[derive(Clone, Copy)]
struct PortSpan {
    start: u16,
    end: u16,
}

impl PortSpan {
    fn single(port: u16) -> Self {
        Self {
            start: port,
            end: port,
        }
    }

    fn range(start: u16, end: u16) -> Option<Self> {
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    fn is_range(self) -> bool {
        self.start != self.end
    }

    fn overlaps(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    fn emit(self) -> String {
        if self.is_range() {
            format!("{}-{}", self.start, self.end)
        } else {
            self.start.to_string()
        }
    }
}

struct MapElem {
    span: PortSpan,
    value: String,
    comment: String,
}

/// RANGE 等宽平移。nftables map 的 value 必须是 singleton，
/// `53051-53080 : 1.2.3.4 . 51051-51080` 会失败，所以走内核原生 interval DNAT。
struct ShiftElem {
    span: PortSpan,
    dest_start: u16,
    dest_end: u16,
    dst_ip: String,
    comment: String,
}

struct SetElem {
    value: String,
    comment: String,
    interval: bool,
}

#[derive(Default)]
struct ProtoSets {
    all: Vec<SetElem>,
    tcp: Vec<SetElem>,
    udp: Vec<SetElem>,
}

impl ProtoSets {
    fn bucket_mut(&mut self, protocol: Protocol) -> &mut Vec<SetElem> {
        match protocol {
            Protocol::All => &mut self.all,
            Protocol::Tcp => &mut self.tcp,
            Protocol::Udp => &mut self.udp,
        }
    }

    fn is_empty(&self) -> bool {
        self.all.is_empty() && self.tcp.is_empty() && self.udp.is_empty()
    }
}

struct NatMaps {
    tcp_dnat: Vec<MapElem>,
    udp_dnat: Vec<MapElem>,
    tcp_dnat_ip: Vec<MapElem>,
    udp_dnat_ip: Vec<MapElem>,
    tcp_redirect: Vec<MapElem>,
    udp_redirect: Vec<MapElem>,
    tcp_shift: Vec<ShiftElem>,
    udp_shift: Vec<ShiftElem>,
    snat: String,
}

impl NatMaps {
    fn new(family: Family) -> Self {
        let snat = match env::var(family.snat_env()) {
            Ok(ip) if !ip.trim().is_empty() => format!("snat to {}", ip.trim()),
            _ => "masquerade".to_string(),
        };
        Self {
            tcp_dnat: Vec::new(),
            udp_dnat: Vec::new(),
            tcp_dnat_ip: Vec::new(),
            udp_dnat_ip: Vec::new(),
            tcp_redirect: Vec::new(),
            udp_redirect: Vec::new(),
            tcp_shift: Vec::new(),
            udp_shift: Vec::new(),
            snat,
        }
    }

    fn is_empty(&self) -> bool {
        self.tcp_dnat.is_empty()
            && self.udp_dnat.is_empty()
            && self.tcp_dnat_ip.is_empty()
            && self.udp_dnat_ip.is_empty()
            && self.tcp_redirect.is_empty()
            && self.udp_redirect.is_empty()
            && self.tcp_shift.is_empty()
            && self.udp_shift.is_empty()
    }

    fn has_dnat(&self) -> bool {
        !self.tcp_dnat.is_empty()
            || !self.udp_dnat.is_empty()
            || !self.tcp_dnat_ip.is_empty()
            || !self.udp_dnat_ip.is_empty()
            || !self.tcp_shift.is_empty()
            || !self.udp_shift.is_empty()
    }

    fn dnat_mut(&mut self, protocol: Protocol) -> &mut Vec<MapElem> {
        match protocol {
            Protocol::Tcp => &mut self.tcp_dnat,
            Protocol::Udp => &mut self.udp_dnat,
            Protocol::All => unreachable!(),
        }
    }

    fn dnat_ip_mut(&mut self, protocol: Protocol) -> &mut Vec<MapElem> {
        match protocol {
            Protocol::Tcp => &mut self.tcp_dnat_ip,
            Protocol::Udp => &mut self.udp_dnat_ip,
            Protocol::All => unreachable!(),
        }
    }

    fn redirect_mut(&mut self, protocol: Protocol) -> &mut Vec<MapElem> {
        match protocol {
            Protocol::Tcp => &mut self.tcp_redirect,
            Protocol::Udp => &mut self.udp_redirect,
            Protocol::All => unreachable!(),
        }
    }

    fn shift_mut(&mut self, protocol: Protocol) -> &mut Vec<ShiftElem> {
        match protocol {
            Protocol::Tcp => &mut self.tcp_shift,
            Protocol::Udp => &mut self.udp_shift,
            Protocol::All => unreachable!(),
        }
    }

    fn port_conflict(&self, protocol: Protocol, span: PortSpan) -> Option<PortSpan> {
        let (dnat, dnat_ip, redirect, shift) = match protocol {
            Protocol::Tcp => (
                &self.tcp_dnat,
                &self.tcp_dnat_ip,
                &self.tcp_redirect,
                &self.tcp_shift,
            ),
            Protocol::Udp => (
                &self.udp_dnat,
                &self.udp_dnat_ip,
                &self.udp_redirect,
                &self.udp_shift,
            ),
            Protocol::All => unreachable!(),
        };
        dnat.iter()
            .map(|e| e.span)
            .chain(dnat_ip.iter().map(|e| e.span))
            .chain(redirect.iter().map(|e| e.span))
            .chain(shift.iter().map(|e| e.span))
            .find(|existing| existing.overlaps(span))
    }
}

#[derive(Default)]
struct FilterSets {
    input_saddr: ProtoSets,
    forward_saddr: ProtoSets,
    input_daddr: ProtoSets,
    forward_daddr: ProtoSets,
    input_dport: ProtoSets,
    forward_dport: ProtoSets,
    input_sport: ProtoSets,
    forward_sport: ProtoSets,
    complex: Vec<(Chain, String, String)>,
}

impl FilterSets {
    fn is_empty(&self) -> bool {
        self.input_saddr.is_empty()
            && self.forward_saddr.is_empty()
            && self.input_daddr.is_empty()
            && self.forward_daddr.is_empty()
            && self.input_dport.is_empty()
            && self.forward_dport.is_empty()
            && self.input_sport.is_empty()
            && self.forward_sport.is_empty()
            && self.complex.is_empty()
    }

    fn needs_chain(&self, chain: Chain) -> bool {
        let in_sets = match chain {
            Chain::Input => {
                !self.input_saddr.is_empty()
                    || !self.input_daddr.is_empty()
                    || !self.input_dport.is_empty()
                    || !self.input_sport.is_empty()
            }
            Chain::Forward => {
                !self.forward_saddr.is_empty()
                    || !self.forward_daddr.is_empty()
                    || !self.forward_dport.is_empty()
                    || !self.forward_sport.is_empty()
            }
        };
        in_sets || self.complex.iter().any(|(c, _, _)| *c == chain)
    }

    fn saddr_mut(&mut self, chain: Chain) -> &mut ProtoSets {
        match chain {
            Chain::Input => &mut self.input_saddr,
            Chain::Forward => &mut self.forward_saddr,
        }
    }

    fn daddr_mut(&mut self, chain: Chain) -> &mut ProtoSets {
        match chain {
            Chain::Input => &mut self.input_daddr,
            Chain::Forward => &mut self.forward_daddr,
        }
    }

    fn dport_mut(&mut self, chain: Chain) -> &mut ProtoSets {
        match chain {
            Chain::Input => &mut self.input_dport,
            Chain::Forward => &mut self.forward_dport,
        }
    }

    fn sport_mut(&mut self, chain: Chain) -> &mut ProtoSets {
        match chain {
            Chain::Input => &mut self.input_sport,
            Chain::Forward => &mut self.forward_sport,
        }
    }
}

struct Ruleset {
    ip4: NatMaps,
    ip6: NatMaps,
    filter4: FilterSets,
    filter6: FilterSets,
}

impl Ruleset {
    fn new() -> Self {
        Self {
            ip4: NatMaps::new(Family::Ip),
            ip6: NatMaps::new(Family::Ip6),
            filter4: FilterSets::default(),
            filter6: FilterSets::default(),
        }
    }

    fn nat_mut(&mut self, family: Family) -> &mut NatMaps {
        match family {
            Family::Ip => &mut self.ip4,
            Family::Ip6 => &mut self.ip6,
        }
    }

    fn filter_mut(&mut self, family: Family) -> &mut FilterSets {
        match family {
            Family::Ip => &mut self.filter4,
            Family::Ip6 => &mut self.filter6,
        }
    }
}

/// 根据运行时配置生成 nftables 脚本。
pub fn build_script(cells: &[RuntimeCell]) -> Result<String, io::Error> {
    let mut ruleset = Ruleset::new();
    for cell in cells {
        if let RuntimeCell::Rule(rule) = cell
            && let Err(e) = add_rule(&mut ruleset, rule)
        {
            warn!("Failed to build rule for {rule:?}: {e}");
        }
    }
    Ok(emit_script(&ruleset))
}

fn add_rule(ruleset: &mut Ruleset, cell: &NftCell) -> Result<(), io::Error> {
    match cell {
        NftCell::Drop { .. } => add_drop(ruleset, cell),
        NftCell::Redirect {
            src_port,
            src_port_end,
            dst_port,
            protocol,
            ip_version,
            comment,
        } => {
            let comment = element_comment(comment.as_deref(), cell);
            let span = match src_port_end {
                Some(end) => PortSpan::range(*src_port, *end).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid redirect port range")
                })?,
                None => PortSpan::single(*src_port),
            };
            let families = match ip_version {
                IpVersion::All => vec![Family::Ip, Family::Ip6],
                IpVersion::V4 => vec![Family::Ip],
                IpVersion::V6 => vec![Family::Ip6],
            };
            for family in families {
                insert_redirect(
                    ruleset.nat_mut(family),
                    *protocol,
                    span,
                    *dst_port,
                    &comment,
                );
            }
            Ok(())
        }
        NftCell::Single {
            sport,
            dport,
            domain,
            protocol,
            ip_version,
            comment,
        } => {
            let (family, dst_ip) = resolve_target(domain, ip_version)?;
            let comment = element_comment(comment.as_deref(), cell);
            if is_localhost(domain, &dst_ip, family) {
                insert_redirect(
                    ruleset.nat_mut(family),
                    *protocol,
                    PortSpan::single(*sport),
                    *dport,
                    &comment,
                );
            } else {
                insert_dnat(
                    ruleset.nat_mut(family),
                    *protocol,
                    PortSpan::single(*sport),
                    &format!("{dst_ip} . {dport}"),
                    &comment,
                );
            }
            Ok(())
        }
        NftCell::Range {
            port_start,
            port_end,
            dport,
            domain,
            protocol,
            ip_version,
            comment,
        } => {
            let (family, dst_ip) = resolve_target(domain, ip_version)?;
            let comment = element_comment(comment.as_deref(), cell);
            let span = PortSpan::range(*port_start, *port_end)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid range ports"))?;
            let (dest_start, dest_end) = range_dnat_ports(*port_start, *port_end, *dport)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            if dest_start == *port_start {
                insert_dnat_ip(ruleset.nat_mut(family), *protocol, span, dst_ip, &comment);
            } else {
                insert_dnat_shift(
                    ruleset.nat_mut(family),
                    *protocol,
                    span,
                    dest_start,
                    dest_end,
                    dst_ip,
                    &comment,
                );
            }
            Ok(())
        }
    }
}

fn resolve_target(domain: &str, ip_version: &IpVersion) -> Result<(Family, String), io::Error> {
    let dst_ip = ip::remote_ip(domain, ip_version)?;
    let parsed: IpAddr = dst_ip.parse().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid IP {dst_ip}: {e}"),
        )
    })?;
    let actual = if parsed.is_ipv6() {
        Family::Ip6
    } else {
        Family::Ip
    };
    match (ip_version, actual) {
        (IpVersion::V4, Family::Ip6) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "IPv6 target address resolved but rule is configured for IPv4 only",
        )),
        (IpVersion::V6, Family::Ip) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "IPv4 target address resolved but rule is configured for IPv6 only",
        )),
        _ => Ok((actual, dst_ip)),
    }
}

fn is_localhost(domain: &str, dst_ip: &str, family: Family) -> bool {
    domain.eq_ignore_ascii_case("localhost") || dst_ip == family.localhost()
}

fn protocols(protocol: Protocol) -> &'static [Protocol] {
    match protocol {
        Protocol::All => &[Protocol::Tcp, Protocol::Udp],
        Protocol::Tcp => &[Protocol::Tcp],
        Protocol::Udp => &[Protocol::Udp],
    }
}

fn insert_redirect(
    maps: &mut NatMaps,
    protocol: Protocol,
    span: PortSpan,
    dst_port: u16,
    comment: &str,
) {
    for proto in protocols(protocol) {
        if skip_conflict(maps, *proto, span, "redirect") {
            continue;
        }
        maps.redirect_mut(*proto).push(MapElem {
            span,
            value: dst_port.to_string(),
            comment: comment.to_string(),
        });
    }
}

fn insert_dnat(maps: &mut NatMaps, protocol: Protocol, span: PortSpan, value: &str, comment: &str) {
    for proto in protocols(protocol) {
        if skip_conflict(maps, *proto, span, "dnat") {
            continue;
        }
        maps.dnat_mut(*proto).push(MapElem {
            span,
            value: value.to_string(),
            comment: comment.to_string(),
        });
    }
}

fn insert_dnat_ip(
    maps: &mut NatMaps,
    protocol: Protocol,
    span: PortSpan,
    dst_ip: String,
    comment: &str,
) {
    for proto in protocols(protocol) {
        if skip_conflict(maps, *proto, span, "range dnat") {
            continue;
        }
        maps.dnat_ip_mut(*proto).push(MapElem {
            span,
            value: dst_ip.clone(),
            comment: comment.to_string(),
        });
    }
}

fn insert_dnat_shift(
    maps: &mut NatMaps,
    protocol: Protocol,
    span: PortSpan,
    dest_start: u16,
    dest_end: u16,
    dst_ip: String,
    comment: &str,
) {
    for proto in protocols(protocol) {
        if skip_conflict(maps, *proto, span, "range shift") {
            continue;
        }
        maps.shift_mut(*proto).push(ShiftElem {
            span,
            dest_start,
            dest_end,
            dst_ip: dst_ip.clone(),
            comment: comment.to_string(),
        });
    }
}

fn skip_conflict(maps: &NatMaps, protocol: Protocol, span: PortSpan, what: &str) -> bool {
    if let Some(existing) = maps.port_conflict(protocol, span) {
        warn!(
            "跳过冲突的{what}: {} 与已有 {} 重叠",
            span.emit(),
            existing.emit()
        );
        true
    } else {
        false
    }
}

fn add_drop(ruleset: &mut Ruleset, cell: &NftCell) -> Result<(), io::Error> {
    let NftCell::Drop {
        chain,
        src_ip,
        dst_ip,
        src_port,
        src_port_end,
        dst_port,
        dst_port_end,
        protocol,
        comment,
    } = cell
    else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected Drop cell",
        ));
    };

    let families = drop_families(src_ip.as_deref(), dst_ip.as_deref())?;
    let comment = element_comment(comment.as_deref(), cell);
    for family in families {
        let filter = ruleset.filter_mut(family);
        let has_src_ip = src_ip.is_some();
        let has_dst_ip = dst_ip.is_some();
        let has_src_port = src_port.is_some();
        let has_dst_port = dst_port.is_some();
        let dims = [has_src_ip, has_dst_ip, has_src_port, has_dst_port]
            .into_iter()
            .filter(|x| *x)
            .count();

        if dims == 1 && has_src_ip {
            if let Some(ip) = src_ip {
                insert_set_elem(
                    filter.saddr_mut(*chain).bucket_mut(*protocol),
                    set_ip_elem(ip, &comment),
                    "drop src_ip",
                );
            }
        } else if dims == 1 && has_dst_ip {
            if let Some(ip) = dst_ip {
                insert_set_elem(
                    filter.daddr_mut(*chain).bucket_mut(*protocol),
                    set_ip_elem(ip, &comment),
                    "drop dst_ip",
                );
            }
        } else if dims == 1 && has_dst_port {
            if let Some(port) = dst_port {
                insert_set_elem(
                    filter.dport_mut(*chain).bucket_mut(*protocol),
                    set_port_elem(*port, *dst_port_end, &comment)?,
                    "drop dst_port",
                );
            }
        } else if dims == 1 && has_src_port {
            if let Some(port) = src_port {
                insert_set_elem(
                    filter.sport_mut(*chain).bucket_mut(*protocol),
                    set_port_elem(*port, *src_port_end, &comment)?,
                    "drop src_port",
                );
            }
        } else {
            filter
                .complex
                .push((*chain, drop_conditions(family, cell)?, comment.clone()));
        }
    }
    Ok(())
}

fn drop_families(src_ip: Option<&str>, dst_ip: Option<&str>) -> Result<Vec<Family>, io::Error> {
    if let Some(ip) = src_ip.or(dst_ip) {
        let network = IpNetwork::from_str(ip).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, format!("无效的IP地址: {ip}"))
        })?;
        if network.is_ipv6() {
            Ok(vec![Family::Ip6])
        } else {
            Ok(vec![Family::Ip])
        }
    } else {
        Ok(vec![Family::Ip, Family::Ip6])
    }
}

fn set_ip_elem(ip: &str, comment: &str) -> SetElem {
    SetElem {
        value: ip.to_string(),
        comment: comment.to_string(),
        interval: ip.contains('/'),
    }
}

fn set_port_elem(start: u16, end: Option<u16>, comment: &str) -> Result<SetElem, io::Error> {
    let span = match end {
        Some(end) => PortSpan::range(start, end)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid drop port range"))?,
        None => PortSpan::single(start),
    };
    Ok(SetElem {
        value: span.emit(),
        comment: comment.to_string(),
        interval: span.is_range(),
    })
}

fn insert_set_elem(list: &mut Vec<SetElem>, elem: SetElem, what: &str) {
    if list.iter().any(|item| item.value == elem.value) {
        warn!("跳过重复的{what}: {}", elem.value);
        return;
    }
    list.push(elem);
}

fn drop_conditions(family: Family, cell: &NftCell) -> Result<String, io::Error> {
    let NftCell::Drop {
        src_ip,
        dst_ip,
        src_port,
        src_port_end,
        dst_port,
        dst_port_end,
        protocol,
        ..
    } = cell
    else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected Drop cell",
        ));
    };

    let mut conditions = Vec::new();
    let prefix = family.name();
    if let Some(ip) = src_ip {
        conditions.push(format!("{prefix} saddr {ip}"));
    }
    if let Some(ip) = dst_ip {
        conditions.push(format!("{prefix} daddr {ip}"));
    }
    if *protocol != Protocol::All || src_port.is_some() || dst_port.is_some() {
        conditions.push(l4_match(*protocol).to_string());
    }
    if let Some(port) = src_port {
        conditions.push(port_match("sport", *port, *src_port_end));
    }
    if let Some(port) = dst_port {
        conditions.push(port_match("dport", *port, *dst_port_end));
    }
    Ok(conditions.join(" "))
}

fn l4_match(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::All => "meta l4proto { tcp, udp } th",
        Protocol::Tcp => "tcp",
        Protocol::Udp => "udp",
    }
}

fn port_match(kind: &str, start: u16, end: Option<u16>) -> String {
    match end {
        Some(end) => format!("{kind} {start}-{end}"),
        None => format!("{kind} {start}"),
    }
}

fn element_comment(user: Option<&str>, cell: &NftCell) -> String {
    let raw = user
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| cell.to_string());
    escape_comment(&raw)
}

fn escape_comment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push('\''),
            '\\' => out.push('/'),
            '\n' | '\r' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
        if out.len() >= 256 {
            break;
        }
    }
    out
}
fn emit_script(ruleset: &Ruleset) -> String {
    let mut out = String::from("#!/usr/sbin/nft -f\n\n");
    out.push_str(
        "# Atomically replace managed tables. Empty tables are kept for listing; hooks are only added when needed.\n",
    );
    for (family, name) in TABLES {
        out.push_str(&format!("add table {family} {name}\n"));
        out.push_str(&format!("delete table {family} {name}\n"));
        out.push_str(&format!("add table {family} {name}\n"));
    }
    out.push('\n');

    emit_nat(&mut out, Family::Ip, &ruleset.ip4);
    emit_nat(&mut out, Family::Ip6, &ruleset.ip6);
    emit_filter(&mut out, Family::Ip, &ruleset.filter4);
    emit_filter(&mut out, Family::Ip6, &ruleset.filter6);
    out
}

fn emit_nat(out: &mut String, family: Family, maps: &NatMaps) {
    if maps.is_empty() {
        return;
    }
    let fam = family.name();
    out.push_str(&format!("# {} NAT\n", fam.to_uppercase()));
    out.push_str(&format!(
        "# ct mark {CT_MARK} marks packets DNATed by this table (ASCII \"NAT\")\n"
    ));

    emit_map(
        out,
        fam,
        "tcp_dnat",
        &format!("inet_service : {} . inet_service", family.addr_type()),
        &maps.tcp_dnat,
    );
    emit_map(
        out,
        fam,
        "udp_dnat",
        &format!("inet_service : {} . inet_service", family.addr_type()),
        &maps.udp_dnat,
    );
    emit_map(
        out,
        fam,
        "tcp_dnat_ip",
        &format!("inet_service : {}", family.addr_type()),
        &maps.tcp_dnat_ip,
    );
    emit_map(
        out,
        fam,
        "udp_dnat_ip",
        &format!("inet_service : {}", family.addr_type()),
        &maps.udp_dnat_ip,
    );
    emit_map(
        out,
        fam,
        "tcp_redirect",
        "inet_service : inet_service",
        &maps.tcp_redirect,
    );
    emit_map(
        out,
        fam,
        "udp_redirect",
        "inet_service : inet_service",
        &maps.udp_redirect,
    );

    out.push_str(&format!(
        "add chain {fam} self-nat PREROUTING {{ type nat hook prerouting priority -110 ; }}\n"
    ));

    emit_redirect_rule(out, fam, "tcp", "tcp_redirect", &maps.tcp_redirect);
    emit_redirect_rule(out, fam, "udp", "udp_redirect", &maps.udp_redirect);
    emit_dnat_rule(
        out,
        fam,
        "tcp",
        "tcp_dnat",
        &maps.tcp_dnat,
        &format!(
            "dnat {} addr . port to tcp dport map @tcp_dnat",
            family.dnat_kw()
        ),
    );
    emit_dnat_rule(
        out,
        fam,
        "udp",
        "udp_dnat",
        &maps.udp_dnat,
        &format!(
            "dnat {} addr . port to udp dport map @udp_dnat",
            family.dnat_kw()
        ),
    );
    emit_dnat_rule(
        out,
        fam,
        "tcp",
        "tcp_dnat_ip",
        &maps.tcp_dnat_ip,
        "dnat to tcp dport map @tcp_dnat_ip",
    );
    emit_dnat_rule(
        out,
        fam,
        "udp",
        "udp_dnat_ip",
        &maps.udp_dnat_ip,
        "dnat to udp dport map @udp_dnat_ip",
    );
    emit_shift_rules(out, family, "tcp", &maps.tcp_shift);
    emit_shift_rules(out, family, "udp", &maps.udp_shift);

    if maps.has_dnat() {
        out.push_str(&format!(
            "add chain {fam} self-nat POSTROUTING {{ type nat hook postrouting priority 110 ; }}\n"
        ));
        out.push_str(&format!(
            "add rule {fam} self-nat POSTROUTING ct mark {CT_MARK} counter {}\n",
            maps.snat
        ));
    }
    out.push('\n');
}

fn map_key_set(name: &str) -> String {
    format!("{name}_k")
}

fn emit_map(out: &mut String, family: &str, name: &str, type_spec: &str, elems: &[MapElem]) {
    if elems.is_empty() {
        return;
    }
    let interval = elems.iter().any(|e| e.span.is_range());
    out.push_str(&format!("add map {family} self-nat {name} {{\n"));
    out.push_str(&format!("    type {type_spec}\n"));
    if interval {
        out.push_str("    flags interval\n");
    }
    out.push_str("    elements = {\n");
    for (i, elem) in elems.iter().enumerate() {
        let comma = if i + 1 == elems.len() { "" } else { "," };
        if elem.comment.is_empty() {
            out.push_str(&format!(
                "        {} : {}{}\n",
                elem.span.emit(),
                elem.value,
                comma
            ));
        } else {
            out.push_str(&format!(
                "        {} comment \"{}\" : {}{}\n",
                elem.span.emit(),
                elem.comment,
                elem.value,
                comma
            ));
        }
    }
    out.push_str("    }\n}\n");
    // Linux < 6.5 rejects treating a named map as a set (`tcp dport @map`).
    // Keep a companion set of the same keys for the match, and use the map
    // only for the DNAT/redirect lookup (`dnat to tcp dport map @map`).
    emit_map_key_set(out, family, &map_key_set(name), elems, interval);
}

fn emit_map_key_set(
    out: &mut String,
    family: &str,
    name: &str,
    elems: &[MapElem],
    interval: bool,
) {
    out.push_str(&format!("add set {family} self-nat {name} {{\n"));
    out.push_str("    type inet_service\n");
    if interval {
        out.push_str("    flags interval\n");
    }
    out.push_str("    elements = {\n");
    for (i, elem) in elems.iter().enumerate() {
        let comma = if i + 1 == elems.len() { "" } else { "," };
        if elem.comment.is_empty() {
            out.push_str(&format!("        {}{}\n", elem.span.emit(), comma));
        } else {
            out.push_str(&format!(
                "        {} comment \"{}\"{}\n",
                elem.span.emit(),
                elem.comment,
                comma
            ));
        }
    }
    out.push_str("    }\n}\n");
}

fn emit_redirect_rule(out: &mut String, family: &str, proto: &str, map: &str, elems: &[MapElem]) {
    if elems.is_empty() {
        return;
    }
    let keys = map_key_set(map);
    out.push_str(&format!(
        "add rule {family} self-nat PREROUTING fib daddr type local {proto} dport @{keys} redirect to {proto} dport map @{map}\n"
    ));
}

fn emit_dnat_rule(
    out: &mut String,
    family: &str,
    proto: &str,
    map: &str,
    elems: &[MapElem],
    dnat: &str,
) {
    if elems.is_empty() {
        return;
    }
    let keys = map_key_set(map);
    out.push_str(&format!(
        "add rule {family} self-nat PREROUTING fib daddr type local {proto} dport @{keys} counter ct mark set {CT_MARK} {dnat}\n"
    ));
}

fn emit_shift_rules(out: &mut String, family: Family, proto: &str, elems: &[ShiftElem]) {
    let fam = family.name();
    for elem in elems {
        let dest_ip = family.fmt_dnat_ip(&elem.dst_ip);
        out.push_str(&format!(
            "add rule {fam} self-nat PREROUTING fib daddr type local {proto} dport {} counter ct mark set {CT_MARK} dnat to {dest_ip}:{}-{} comment \"{}\"\n",
            elem.span.emit(),
            elem.dest_start,
            elem.dest_end,
            elem.comment,
        ));
    }
}

fn emit_filter(out: &mut String, family: Family, filter: &FilterSets) {
    if filter.is_empty() {
        return;
    }
    let fam = family.name();
    out.push_str(&format!("# {} filter\n", fam.to_uppercase()));

    let addr_type = family.addr_type();
    emit_proto_sets(out, fam, "input_saddr", addr_type, &filter.input_saddr);
    emit_proto_sets(out, fam, "forward_saddr", addr_type, &filter.forward_saddr);
    emit_proto_sets(out, fam, "input_daddr", addr_type, &filter.input_daddr);
    emit_proto_sets(out, fam, "forward_daddr", addr_type, &filter.forward_daddr);
    emit_proto_sets(out, fam, "input_dport", "inet_service", &filter.input_dport);
    emit_proto_sets(
        out,
        fam,
        "forward_dport",
        "inet_service",
        &filter.forward_dport,
    );
    emit_proto_sets(out, fam, "input_sport", "inet_service", &filter.input_sport);
    emit_proto_sets(
        out,
        fam,
        "forward_sport",
        "inet_service",
        &filter.forward_sport,
    );

    if filter.needs_chain(Chain::Input) {
        out.push_str(&format!(
            "add chain {fam} self-filter INPUT {{ type filter hook input priority filter - 1 ; }}\n"
        ));
    }
    if filter.needs_chain(Chain::Forward) {
        out.push_str(&format!(
            "add chain {fam} self-filter FORWARD {{ type filter hook forward priority filter - 1 ; }}\n"
        ));
    }

    emit_drop_set_rules(out, fam, family.name(), Chain::Input, filter);
    emit_drop_set_rules(out, fam, family.name(), Chain::Forward, filter);

    for (chain, conditions, comment) in &filter.complex {
        let chain_name = chain_name(*chain);
        if comment.is_empty() {
            out.push_str(&format!(
                "add rule {fam} self-filter {chain_name} {conditions} counter drop\n"
            ));
        } else {
            out.push_str(&format!(
                "add rule {fam} self-filter {chain_name} {conditions} counter drop comment \"{comment}\"\n"
            ));
        }
    }
    out.push('\n');
}

fn emit_proto_sets(
    out: &mut String,
    family: &str,
    prefix: &str,
    type_spec: &str,
    sets: &ProtoSets,
) {
    emit_set(out, family, prefix, type_spec, &sets.all);
    emit_set(out, family, &format!("{prefix}_tcp"), type_spec, &sets.tcp);
    emit_set(out, family, &format!("{prefix}_udp"), type_spec, &sets.udp);
}

fn emit_set(out: &mut String, family: &str, name: &str, type_spec: &str, elems: &[SetElem]) {
    if elems.is_empty() {
        return;
    }
    let interval = elems.iter().any(|e| e.interval);
    out.push_str(&format!("add set {family} self-filter {name} {{\n"));
    out.push_str(&format!("    type {type_spec}\n"));
    if interval {
        out.push_str("    flags interval\n");
    }
    out.push_str("    elements = {\n");
    for (i, elem) in elems.iter().enumerate() {
        let comma = if i + 1 == elems.len() { "" } else { "," };
        if elem.comment.is_empty() {
            out.push_str(&format!("        {}{}\n", elem.value, comma));
        } else {
            out.push_str(&format!(
                "        {} comment \"{}\"{}\n",
                elem.value, elem.comment, comma
            ));
        }
    }
    out.push_str("    }\n}\n");
}

fn emit_drop_set_rules(
    out: &mut String,
    fam: &str,
    ip_prefix: &str,
    chain: Chain,
    filter: &FilterSets,
) {
    let chain_name = chain_name(chain);
    let (saddr, daddr, dport, sport) = match chain {
        Chain::Input => (
            &filter.input_saddr,
            &filter.input_daddr,
            &filter.input_dport,
            &filter.input_sport,
        ),
        Chain::Forward => (
            &filter.forward_saddr,
            &filter.forward_daddr,
            &filter.forward_dport,
            &filter.forward_sport,
        ),
    };
    let prefix = match chain {
        Chain::Input => "input",
        Chain::Forward => "forward",
    };

    emit_addr_drop_rules(out, fam, chain_name, ip_prefix, "saddr", prefix, saddr);
    emit_addr_drop_rules(out, fam, chain_name, ip_prefix, "daddr", prefix, daddr);
    emit_port_drop_rules(out, fam, chain_name, "dport", prefix, dport);
    emit_port_drop_rules(out, fam, chain_name, "sport", prefix, sport);
}

fn emit_addr_drop_rules(
    out: &mut String,
    fam: &str,
    chain_name: &str,
    ip_prefix: &str,
    kind: &str,
    prefix: &str,
    sets: &ProtoSets,
) {
    let set = format!("{prefix}_{kind}");
    if !sets.all.is_empty() {
        out.push_str(&format!(
            "add rule {fam} self-filter {chain_name} {ip_prefix} {kind} @{set} counter drop\n"
        ));
    }
    if !sets.tcp.is_empty() {
        out.push_str(&format!(
            "add rule {fam} self-filter {chain_name} {ip_prefix} {kind} @{set}_tcp tcp counter drop\n"
        ));
    }
    if !sets.udp.is_empty() {
        out.push_str(&format!(
            "add rule {fam} self-filter {chain_name} {ip_prefix} {kind} @{set}_udp udp counter drop\n"
        ));
    }
}

fn emit_port_drop_rules(
    out: &mut String,
    fam: &str,
    chain_name: &str,
    kind: &str,
    prefix: &str,
    sets: &ProtoSets,
) {
    let set = format!("{prefix}_{kind}");
    if !sets.all.is_empty() {
        out.push_str(&format!(
            "add rule {fam} self-filter {chain_name} meta l4proto {{ tcp, udp }} th {kind} @{set} counter drop\n"
        ));
    }
    if !sets.tcp.is_empty() {
        out.push_str(&format!(
            "add rule {fam} self-filter {chain_name} tcp {kind} @{set}_tcp counter drop\n"
        ));
    }
    if !sets.udp.is_empty() {
        out.push_str(&format!(
            "add rule {fam} self-filter {chain_name} udp {kind} @{set}_udp counter drop\n"
        ));
    }
}

fn chain_name(chain: Chain) -> &'static str {
    match chain {
        Chain::Input => "INPUT",
        Chain::Forward => "FORWARD",
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeCell;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn rule(cell: NftCell) -> RuntimeCell {
        RuntimeCell::Rule(cell)
    }

    fn check_nft(script: &str) {
        if !Path::new("/usr/sbin/nft").exists() {
            return;
        }
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "nftables-nat-rust-{}-{unique}.nft",
            std::process::id()
        ));
        fs::write(&path, script).unwrap();
        let output = Command::new("/usr/sbin/nft")
            .arg("-c")
            .arg("-f")
            .arg(&path)
            .output()
            .unwrap();
        let _ = fs::remove_file(&path);
        assert!(
            output.status.success(),
            "nft -c failed:\n{}\nstderr:\n{}\nscript:\n{script}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_build_redirect_single_ipv4() {
        let script = build_script(&[rule(NftCell::Redirect {
            src_port: 8000,
            src_port_end: None,
            dst_port: 3128,
            protocol: Protocol::All,
            ip_version: IpVersion::V4,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("add map ip self-nat tcp_redirect"));
        assert!(script.contains("add set ip self-nat tcp_redirect_k"));
        assert!(script.contains("8000 comment \"REDIRECT,8000,3128,all,ipv4\" : 3128"));
        assert!(script.contains(
            "add rule ip self-nat PREROUTING fib daddr type local tcp dport @tcp_redirect_k redirect to tcp dport map @tcp_redirect"
        ));
        assert!(script.contains(
            "add rule ip self-nat PREROUTING fib daddr type local udp dport @udp_redirect_k redirect to udp dport map @udp_redirect"
        ));
        assert!(!script.contains("add map ip6"));
        assert!(!script.contains("POSTROUTING"));
        check_nft(&script);
    }

    #[test]
    fn test_build_redirect_range_ipv4() {
        let script = build_script(&[rule(NftCell::Redirect {
            src_port: 30001,
            src_port_end: Some(39999),
            dst_port: 45678,
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("flags interval"));
        assert!(
            script.contains("30001-39999 comment \"REDIRECT,30001-39999,45678,tcp,ipv4\" : 45678")
        );
        assert!(!script.contains("udp_redirect"));
        check_nft(&script);
    }

    #[test]
    fn test_build_redirect_both_ipv() {
        let script = build_script(&[rule(NftCell::Redirect {
            src_port: 5000,
            src_port_end: None,
            dst_port: 4000,
            protocol: Protocol::All,
            ip_version: IpVersion::All,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("add map ip self-nat tcp_redirect"));
        assert!(script.contains("add map ip6 self-nat tcp_redirect"));
        check_nft(&script);
    }

    #[test]
    fn test_build_single_dnat_uses_map_and_mark() {
        let script = build_script(&[rule(NftCell::Single {
            sport: 10000,
            dport: 443,
            domain: "1.2.3.4".to_string(),
            protocol: Protocol::All,
            ip_version: IpVersion::V4,
            comment: Some("web".to_string()),
        })])
        .unwrap();
        assert!(script.contains("10000 comment \"web\" : 1.2.3.4 . 443"));
        assert!(script.contains("add set ip self-nat tcp_dnat_k"));
        assert!(script.contains("fib daddr type local tcp dport @tcp_dnat_k counter ct mark set 0x4e4154 dnat ip addr . port to tcp dport map @tcp_dnat"));
        assert!(script.contains("ct mark 0x4e4154 counter masquerade"));
        assert!(!script.contains("add map ip6"));
        assert!(!script.contains("add chain ip self-filter"));
        check_nft(&script);
    }

    #[test]
    fn test_build_range_preserves_port() {
        let script = build_script(&[rule(NftCell::Range {
            port_start: 1000,
            port_end: 2000,
            dport: None,
            domain: "5.6.7.8".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("add map ip self-nat tcp_dnat_ip"));
        assert!(script.contains("add set ip self-nat tcp_dnat_ip_k"));
        assert!(
            script.contains("1000-2000 comment \"RANGE,1000,2000,5.6.7.8,tcp,ipv4\" : 5.6.7.8")
        );
        assert!(script.contains("dnat to tcp dport map @tcp_dnat_ip"));
        assert!(script.contains("tcp dport @tcp_dnat_ip_k"));
        assert!(!script.contains("tcp_dnat {"));
        check_nft(&script);
    }

    #[test]
    fn test_localhost_becomes_redirect() {
        let script = build_script(&[rule(NftCell::Single {
            sport: 2222,
            dport: 22,
            domain: "localhost".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("tcp_redirect"));
        assert!(!script.contains("tcp_dnat"));
        assert!(!script.contains("POSTROUTING"));
        check_nft(&script);
    }

    #[test]
    fn test_ipv6_dnat_and_empty_ipv4() {
        let script = build_script(&[rule(NftCell::Single {
            sport: 9001,
            dport: 9099,
            domain: "2001:db8::1".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V6,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("add map ip6 self-nat tcp_dnat"));
        assert!(script.contains("add set ip6 self-nat tcp_dnat_k"));
        assert!(script.contains(
            "9001 comment \"SINGLE,9001,9099,2001:db8::1,tcp,ipv6\" : 2001:db8::1 . 9099"
        ));
        assert!(script.contains("dnat ip6 addr . port to tcp dport map @tcp_dnat"));
        assert!(script.contains("tcp dport @tcp_dnat_k"));
        assert!(!script.contains("add map ip self-nat"));
        check_nft(&script);
    }

    #[test]
    fn test_drop_src_ip_uses_set() {
        let script = build_script(&[rule(NftCell::Drop {
            chain: Chain::Input,
            src_ip: Some("8.8.8.0/24".to_string()),
            dst_ip: None,
            src_port: None,
            src_port_end: None,
            dst_port: None,
            dst_port_end: None,
            protocol: Protocol::All,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("add set ip self-filter input_saddr"));
        assert!(script.contains("flags interval"));
        assert!(script.contains("8.8.8.0/24"));
        assert!(
            script.contains("add rule ip self-filter INPUT ip saddr @input_saddr counter drop")
        );
        assert!(!script.contains("add chain ip self-filter FORWARD"));
        assert!(!script.contains("add set ip6"));
        check_nft(&script);
    }

    #[test]
    fn test_drop_port_and_combo() {
        let script = build_script(&[
            rule(NftCell::Drop {
                chain: Chain::Input,
                src_ip: None,
                dst_ip: None,
                src_port: None,
                src_port_end: None,
                dst_port: Some(22),
                dst_port_end: None,
                protocol: Protocol::Tcp,
                comment: Some("ssh".to_string()),
            }),
            rule(NftCell::Drop {
                chain: Chain::Input,
                src_ip: Some("192.168.1.0/24".to_string()),
                dst_ip: None,
                src_port: None,
                src_port_end: None,
                dst_port: Some(3306),
                dst_port_end: None,
                protocol: Protocol::Tcp,
                comment: Some("mysql".to_string()),
            }),
        ])
        .unwrap();
        assert!(script.contains("add set ip self-filter input_dport_tcp"));
        assert!(script.contains("22 comment \"ssh\""));
        assert!(script.contains(
            "add rule ip self-filter INPUT ip saddr 192.168.1.0/24 tcp dport 3306 counter drop comment \"mysql\""
        ));
        check_nft(&script);
    }

    #[test]
    fn test_comment_is_escaped() {
        let script = build_script(&[rule(NftCell::Single {
            sport: 80,
            dport: 8080,
            domain: "10.0.0.1".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: Some(r#"say "hi""#.to_string()),
        })])
        .unwrap();
        assert!(script.contains(r#"80 comment "say 'hi'" : 10.0.0.1 . 8080"#));
        check_nft(&script);
    }

    #[test]
    fn test_build_range_shift_uses_interval_dnat() {
        let script = build_script(&[rule(NftCell::Range {
            port_start: 53051,
            port_end: 53080,
            dport: Some(51051),
            domain: "123.123.123.123".to_string(),
            protocol: Protocol::All,
            ip_version: IpVersion::V4,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains(
            "add rule ip self-nat PREROUTING fib daddr type local tcp dport 53051-53080 counter ct mark set 0x4e4154 dnat to 123.123.123.123:51051-51080 comment \"RANGE,53051,53080,51051,123.123.123.123,all,ipv4\""
        ));
        assert!(script.contains(
            "add rule ip self-nat PREROUTING fib daddr type local udp dport 53051-53080 counter ct mark set 0x4e4154 dnat to 123.123.123.123:51051-51080 comment \"RANGE,53051,53080,51051,123.123.123.123,all,ipv4\""
        ));
        assert!(script.contains("ct mark 0x4e4154"));
        assert!(!script.contains("tcp_dnat_ip"));
        assert!(!script.contains("51051-51080 :"));
        check_nft(&script);
    }

    #[test]
    fn test_range_identity_dport_still_uses_map() {
        let script = build_script(&[rule(NftCell::Range {
            port_start: 1000,
            port_end: 2000,
            dport: Some(1000),
            domain: "5.6.7.8".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V4,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains("add map ip self-nat tcp_dnat_ip"));
        assert!(script.contains("add set ip self-nat tcp_dnat_ip_k"));
        assert!(script.contains("dnat to tcp dport map @tcp_dnat_ip"));
        assert!(script.contains("tcp dport @tcp_dnat_ip_k"));
        assert!(!script.contains("dnat to 5.6.7.8:1000-2000"));
        check_nft(&script);
    }

    #[test]
    fn test_ipv6_range_shift() {
        let script = build_script(&[rule(NftCell::Range {
            port_start: 20001,
            port_end: 20010,
            dport: Some(10001),
            domain: "2001:db8::1".to_string(),
            protocol: Protocol::Tcp,
            ip_version: IpVersion::V6,
            comment: None,
        })])
        .unwrap();
        assert!(script.contains(
            "dnat to [2001:db8::1]:10001-10010 comment \"RANGE,20001,20010,10001,2001:db8::1,tcp,ipv6\""
        ));
        assert!(!script.contains("add map ip6 self-nat tcp_dnat_ip"));
        check_nft(&script);
    }

    #[test]
    fn test_no_rules_only_deletes_tables() {
        let script = build_script(&[]).unwrap();
        assert!(script.contains("delete table ip self-nat"));
        assert!(script.contains("add table ip self-nat"));
        assert!(!script.contains("add chain"));
        assert!(!script.contains("add map"));
        check_nft(&script);
    }
}
