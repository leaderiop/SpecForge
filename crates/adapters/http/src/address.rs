//! Which addresses a request that does not allow private ones may reach, and the resolver that holds it to them.

use std::error::Error as StdError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs as _};
use std::sync::mpsc;
use std::time::Duration;

use reqwest::Url;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// The IPv4 ranges such a request never reaches (RFC 6890's local and special ranges, multicast).
const V4_DENIED: &[(Ipv4Addr, u8)] = &[
    (Ipv4Addr::new(0, 0, 0, 0), 8), // "this network", the unspecified address
    (Ipv4Addr::new(10, 0, 0, 0), 8), // private
    (Ipv4Addr::new(100, 64, 0, 0), 10), // carrier-grade NAT
    (Ipv4Addr::new(127, 0, 0, 0), 8), // loopback
    (Ipv4Addr::new(169, 254, 0, 0), 16), // link-local
    (Ipv4Addr::new(172, 16, 0, 0), 12), // private
    (Ipv4Addr::new(192, 168, 0, 0), 16), // private
    (Ipv4Addr::new(224, 0, 0, 0), 4), // multicast
    (Ipv4Addr::new(240, 0, 0, 0), 4), // reserved, the broadcast address with it
];

/// What an IPv6 range is to such a request.
#[derive(Debug, Clone, Copy)]
enum V6 {
    Denied,
    /// Judged by the IPv4 address it carries, `shift` bits from the right.
    Carries {
        shift: u32,
    },
}

/// The IPv6 ranges that are not plainly public, the first match deciding.
const V6_RANGES: &[(Ipv6Addr, u8, V6)] = &[
    (Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 128, V6::Denied), // unspecified
    (Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 128, V6::Denied), // loopback
    (
        Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0, 0),
        96,
        V6::Carries { shift: 0 },
    ), // IPv4-mapped
    (
        Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
        96,
        V6::Carries { shift: 0 },
    ), // IPv4-compatible
    (
        Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 0, 0),
        96,
        V6::Carries { shift: 0 },
    ), // NAT64
    (
        Ipv6Addr::new(0x64, 0xff9b, 1, 0, 0, 0, 0, 0),
        48,
        V6::Denied,
    ), // local-use NAT64
    (
        Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0),
        16,
        V6::Carries { shift: 80 },
    ), // 6to4
    (Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0), 7, V6::Denied), // unique local
    (Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0), 10, V6::Denied), // link-local
    (Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0), 10, V6::Denied), // site-local
    (Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0), 8, V6::Denied), // multicast
];

/// Whether `addr` is in the `prefix`-bit network `net`, both `width` bits wide.
fn within(addr: u128, net: u128, prefix: u8, width: u32) -> bool {
    ((addr ^ net).checked_shr(width - u32::from(prefix))).unwrap_or(0) == 0
}

/// An address anyone on the internet could reach: in none of the denied ranges, and an IPv6 address that carries an
/// IPv4 one (mapped, compatible, NAT64, 6to4) judged by the address it carries.
pub fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => !V4_DENIED
            .iter()
            .any(|(net, prefix)| within(v4.to_bits().into(), net.to_bits().into(), *prefix, 32)),
        IpAddr::V6(v6) => {
            let bits = v6.to_bits();
            match V6_RANGES
                .iter()
                .find(|(net, prefix, _)| within(bits, net.to_bits(), *prefix, 128))
            {
                None => true,
                Some((_, _, V6::Denied)) => false,
                Some((_, _, V6::Carries { shift })) => {
                    public(IpAddr::V4(Ipv4Addr::from_bits((bits >> shift) as u32)))
                }
            }
        }
    }
}

/// Resolves a name once and keeps its public addresses, the client connecting to one of those; a proxy's own host
/// is exempt, for a proxy on the local network is how many reach the internet at all.
#[derive(Clone)]
pub(crate) struct PublicOnly {
    exempt: Vec<String>,
    lookup: fn(&str) -> std::io::Result<Vec<SocketAddr>>,
}

impl PublicOnly {
    /// The system resolver, the proxies the environment names exempt.
    // shortcut: a proxy set only in the macOS or Windows system settings (reqwest's `system-proxy` reads them) is
    // not exempt, so a private one there is refused; read them here when reqwest exposes its proxy list.
    pub(crate) fn system() -> Self {
        let named = ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"]
            .into_iter()
            .flat_map(|name| [name.to_string(), name.to_lowercase()])
            .filter_map(|name| std::env::var(name).ok());
        PublicOnly {
            exempt: proxy_hosts(named),
            lookup: |host| (host, 0).to_socket_addrs().map(Iterator::collect),
        }
    }

    /// The addresses of `host` a request may connect to: every one of a proxy's, else the public ones.
    fn addresses(&self, host: &str) -> Result<Vec<SocketAddr>, Failed> {
        let all = (self.lookup)(host).map_err(|e| Failed::Dns(e.to_string()))?;
        if self
            .exempt
            .iter()
            .any(|proxy| proxy.eq_ignore_ascii_case(host))
        {
            return Ok(all);
        }
        let kept: Vec<SocketAddr> = all.into_iter().filter(|a| public(a.ip())).collect();
        if kept.is_empty() {
            return Err(Failed::Refused(host.to_string()));
        }
        Ok(kept)
    }
}

/// A destination's name judged on this machine, before anything is sent: through a proxy the proxy resolves it, and
/// the client's resolver only ever sees the proxy's own host. `Refused` when it resolves here only to addresses a
/// request may not reach, a proxy's own host included. A name that does not resolve here within `within` is not
/// refused: on a network whose only DNS is the proxy's that is every name, and on a direct request the resolver
/// answers for it.
// shortcut: a direct request looks its name up twice, for reqwest does not say whether a proxy will carry a request;
// judge only the proxied ones when it does.
pub(crate) fn destination(host: &str, within: Duration) -> Result<(), Failed> {
    judged_destination(host, within, |host| {
        (host, 0).to_socket_addrs().map(Iterator::collect)
    })
}

fn judged_destination(
    host: &str,
    within: Duration,
    lookup: fn(&str) -> std::io::Result<Vec<SocketAddr>>,
) -> Result<(), Failed> {
    let (sent, answer) = mpsc::channel();
    let name = host.to_string();
    // The system resolver blocks with no limit of its own: a slow name must not outlast the request's timeout.
    std::thread::spawn(move || sent.send(lookup(&name)));
    match answer.recv_timeout(within) {
        Ok(Ok(all)) if !all.is_empty() && !all.iter().any(|a| public(a.ip())) => {
            Err(Failed::Refused(host.to_string()))
        }
        _ => Ok(()),
    }
}

/// The hosts of the proxy URLs in `values`, a bare `host:port` read as `http://` the way reqwest reads it.
fn proxy_hosts(values: impl Iterator<Item = String>) -> Vec<String> {
    values
        .filter_map(|value| {
            let url = Url::parse(&value)
                .ok()
                .filter(|url| url.host_str().is_some())
                .or_else(|| Url::parse(&format!("http://{value}")).ok())?;
            url.host_str().map(str::to_string)
        })
        .collect()
}

impl Resolve for PublicOnly {
    fn resolve(&self, name: Name) -> Resolving {
        let this = self.clone();
        let host = name.as_str().to_string();
        Box::pin(async move {
            // The system resolver blocks: off the client's runtime thread, a slow name delays only its own request.
            let kept = tokio::task::spawn_blocking(move || this.addresses(&host))
                .await
                .map_err(|e| Box::new(e) as BoxError)?
                .map_err(|e| Box::new(e) as BoxError)?;
            Ok(Box::new(kept.into_iter()) as Addrs)
        })
    }
}

/// What a resolver fails with.
type BoxError = Box<dyn StdError + Send + Sync>;

/// Why the resolver gave no address.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Failed {
    Dns(String),
    Refused(String),
}

impl std::fmt::Display for Failed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failed::Dns(why) => write!(f, "the name did not resolve: {why}"),
            Failed::Refused(host) => write!(
                f,
                "{host} resolves only to loopback, private, link-local or unspecified addresses"
            ),
        }
    }
}

impl StdError for Failed {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_denied_range_is_not_public() {
        for ip in [
            "0.1.2.3",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.168.1.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::10.0.0.1",
            "64:ff9b::10.0.0.1",
            "64:ff9b:1::1",
            "2002:0a00:0001::1",
            "fd00::1",
            "fe80::1",
            "fec0::1",
            "ff02::1",
        ] {
            assert!(!public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn a_public_address_is_public_however_it_is_carried() {
        for ip in [
            "93.184.215.14",
            "100.128.0.1",
            "172.32.0.1",
            "2606:4700::1111",
            "::ffff:93.184.215.14",
            "64:ff9b::93.184.215.14",
            "2002:5db8:d70e::1",
        ] {
            assert!(public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn a_proxys_own_host_is_exempt_and_no_other() {
        let private: SocketAddr = "10.0.0.8:0".parse().unwrap();
        let resolver = PublicOnly {
            exempt: proxy_hosts(["http://proxy.corp:3128".to_string()].into_iter()),
            lookup: |_| Ok(vec!["10.0.0.8:0".parse().unwrap()]),
        };
        assert_eq!(resolver.addresses("PROXY.corp"), Ok(vec![private]));
        assert_eq!(
            resolver.addresses("intranet.corp"),
            Err(Failed::Refused("intranet.corp".into()))
        );
        let mixed = PublicOnly {
            exempt: Vec::new(),
            lookup: |_| {
                Ok(vec![
                    "10.0.0.8:0".parse().unwrap(),
                    "93.184.215.14:0".parse().unwrap(),
                ])
            },
        };
        assert_eq!(
            mixed.addresses("a.example"),
            Ok(vec!["93.184.215.14:0".parse().unwrap()])
        );
    }

    #[test]
    fn a_destination_is_refused_only_for_what_it_resolves_to_here() {
        let within = Duration::from_secs(5);
        let judged = |lookup| judged_destination("a.example", within, lookup);
        assert_eq!(
            judged(|_| Ok(vec!["10.0.0.8:0".parse().unwrap()])),
            Err(Failed::Refused("a.example".into()))
        );
        assert_eq!(
            judged(|_| Ok(vec!["93.184.215.14:0".parse().unwrap()])),
            Ok(())
        );
        // As a direct request's resolver keeps the public ones.
        assert_eq!(
            judged(|_| Ok(vec![
                "10.0.0.8:0".parse().unwrap(),
                "93.184.215.14:0".parse().unwrap()
            ])),
            Ok(())
        );
        assert_eq!(
            judged(|_| Err(std::io::Error::other("no DNS here"))),
            Ok(())
        );
        assert_eq!(judged(|_| Ok(Vec::new())), Ok(()));
    }

    #[test]
    fn a_name_that_will_not_resolve_in_time_does_not_hold_the_request() {
        let started = std::time::Instant::now();
        let slow = judged_destination("a.example", Duration::from_millis(50), |_| {
            std::thread::sleep(Duration::from_secs(2));
            Ok(vec!["10.0.0.8:0".parse().unwrap()])
        });
        assert_eq!(slow, Ok(()));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_proxy_host_is_read_with_or_without_a_scheme() {
        let hosts = proxy_hosts(
            [
                "http://p.corp:3128",
                "q.corp:8080",
                "socks5://[::1]:1080",
                "::nonsense",
            ]
            .map(str::to_string)
            .into_iter(),
        );
        assert_eq!(hosts, ["p.corp", "q.corp", "[::1]"]);
    }
}
