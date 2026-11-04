//! What the Http port answers beside a request: a name's addresses, and a handshake's facts about a host's certificate.
//! Both hold to the request's time, and neither hands out an address the request does not allow.

use std::io;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme};
use specforge_effects::http::{CONNECT_TIMEOUT, TIMEOUT_CEILING};
use specforge_effects::{
    OutcomeClass, ResolveOutcome, ResolveRequest, TlsOutcome, TlsRequest, Unreachable,
};

use crate::address::public;

/// `host`'s addresses, at most after `time`: the system resolver takes no timeout, so it runs beside the wait and is
/// left behind when the time runs out.
fn lookup(host: &str, port: u16, time: Duration) -> Result<Vec<SocketAddr>, Unreachable> {
    let (send, answer) = mpsc::channel();
    let name = (host.to_string(), port);
    std::thread::spawn(move || {
        let looked = name.to_socket_addrs().map(Iterator::collect::<Vec<_>>);
        let _ = send.send(looked);
    });
    match answer.recv_timeout(time) {
        Ok(Ok(addresses)) if !addresses.is_empty() => Ok(addresses),
        Ok(_) => Err(Unreachable::Dns),
        Err(_) => Err(Unreachable::Timeout),
    }
}

/// `host`'s addresses the request allows: none left is `refused_address`.
fn allowed(
    host: &str,
    port: u16,
    allow_private: bool,
    time: Duration,
) -> Result<Vec<SocketAddr>, Unreachable> {
    let kept: Vec<SocketAddr> = lookup(host, port, time)?
        .into_iter()
        .filter(|address| allow_private || public(address.ip()))
        .collect();
    if kept.is_empty() {
        return Err(Unreachable::RefusedAddress);
    }
    Ok(kept)
}

fn time_of(timeout_ms: u64) -> Duration {
    Duration::from_millis(timeout_ms).min(TIMEOUT_CEILING)
}

pub(crate) fn resolve(request: &ResolveRequest) -> ResolveOutcome {
    let time = time_of(request.timeout_ms);
    match allowed(&request.host, 0, request.allow_private, time) {
        Ok(addresses) => {
            let mut addresses: Vec<String> =
                addresses.iter().map(|at| at.ip().to_string()).collect();
            // One answer however the resolver orders it: what a verdict digests.
            addresses.sort();
            addresses.dedup();
            ResolveOutcome {
                class: OutcomeClass::Ok,
                addresses,
            }
        }
        Err(why) => ResolveOutcome::of(OutcomeClass::Unreachable(why)),
    }
}

pub(crate) fn tls(config: ClientConfig, request: &TlsRequest) -> TlsOutcome {
    let failed = |why| TlsOutcome::of(OutcomeClass::Unreachable(why));
    let deadline = Instant::now() + time_of(request.timeout_ms);
    let addresses = match allowed(
        &request.host,
        request.tcp_port,
        request.allow_private,
        time_of(request.timeout_ms),
    ) {
        Ok(addresses) => addresses,
        Err(why) => return failed(why),
    };
    let Ok(name) = ServerName::try_from(request.host.clone()) else {
        return failed(Unreachable::Connect);
    };
    match shake(Arc::new(config), &name, &addresses, deadline) {
        Ok(leaf) => {
            let (not_after, issuer) = leaf.as_deref().map_or((None, None), facts);
            TlsOutcome {
                class: OutcomeClass::Ok,
                not_after,
                issuer,
                chain_valid: true,
            }
        }
        Err(Shaken::Failed(why)) => failed(why),
        Err(Shaken::Rejected) => {
            // Read again by a verifier that accepts nothing: the facts of what the port's verifier refused.
            let seen = Arc::new(Reading::default());
            let (not_after, issuer) = reading_config(&seen)
                .ok()
                .and_then(|config| {
                    let _ = shake(Arc::new(config), &name, &addresses, deadline);
                    seen.leaf.lock().unwrap_or_else(|e| e.into_inner()).take()
                })
                .map_or((None, None), |leaf| facts(&leaf));
            TlsOutcome {
                class: OutcomeClass::Unreachable(Unreachable::Tls),
                not_after,
                issuer,
                chain_valid: false,
            }
        }
    }
}

/// Why a handshake ended.
enum Shaken {
    /// The verifier refused the certificate.
    Rejected,
    Failed(Unreachable),
}

/// One handshake with the first address that connects; the leaf certificate's bytes.
fn shake(
    config: Arc<ClientConfig>,
    name: &ServerName<'static>,
    addresses: &[SocketAddr],
    deadline: Instant,
) -> Result<Option<Vec<u8>>, Shaken> {
    let left = || deadline.saturating_duration_since(Instant::now());
    let mut stream = None;
    for address in addresses {
        let time = left().min(CONNECT_TIMEOUT);
        if time.is_zero() {
            return Err(Shaken::Failed(Unreachable::Timeout));
        }
        if let Ok(connected) = TcpStream::connect_timeout(address, time) {
            stream = Some(connected);
            break;
        }
    }
    let Some(mut stream) = stream else {
        return Err(Shaken::Failed(Unreachable::Connect));
    };
    let mut connection = ClientConnection::new(config, name.clone())
        .map_err(|_| Shaken::Failed(Unreachable::Tls))?;
    while connection.is_handshaking() {
        let time = left();
        if time.is_zero() {
            return Err(Shaken::Failed(Unreachable::Timeout));
        }
        let timed = stream
            .set_read_timeout(Some(time))
            .and_then(|()| stream.set_write_timeout(Some(time)));
        timed.map_err(|_| Shaken::Failed(Unreachable::Reset))?;
        // One step a turn, not `complete_io`: that keeps reading while a host drips, and each byte resets the timeout.
        if connection.wants_write() {
            connection.write_tls(&mut stream).map_err(shaken_by)?;
            continue;
        }
        if connection.read_tls(&mut stream).map_err(shaken_by)? == 0 {
            return Err(Shaken::Failed(Unreachable::Reset));
        }
        connection
            .process_new_packets()
            .map_err(|refused| shaken_by(io::Error::new(io::ErrorKind::InvalidData, refused)))?;
    }
    Ok(connection
        .peer_certificates()
        .and_then(<[CertificateDer<'_>]>::first)
        .map(|leaf| leaf.to_vec()))
}

fn shaken_by(error: io::Error) -> Shaken {
    let refused = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<rustls::Error>());
    match (refused, error.kind()) {
        (Some(rustls::Error::InvalidCertificate(_)), _) => Shaken::Rejected,
        (Some(_), _) => Shaken::Failed(Unreachable::Tls),
        (None, io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) => {
            Shaken::Failed(Unreachable::Timeout)
        }
        (None, _) => Shaken::Failed(Unreachable::Reset),
    }
}

/// A verifier that notes the leaf and accepts nothing.
#[derive(Debug, Default)]
struct Reading {
    leaf: Mutex<Option<Vec<u8>>>,
}

impl ServerCertVerifier for Reading {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        *self.leaf.lock().unwrap_or_else(|e| e.into_inner()) = Some(end_entity.to_vec());
        Err(rustls::Error::General("read only".to_string()))
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("read only".to_string()))
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("read only".to_string()))
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn reading_config(seen: &Arc<Reading>) -> Result<ClientConfig, rustls::Error> {
    Ok(
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(Arc::clone(seen) as Arc<dyn ServerCertVerifier>)
            .with_no_client_auth(),
    )
}

/// One DER element: its tag, content and what follows it.
fn element(der: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = der.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (length, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let (bytes, rest) = rest.split_at(count);
        let length = bytes
            .iter()
            .fold(0usize, |length, byte| (length << 8) | usize::from(*byte));
        (length, rest)
    };
    (length <= rest.len()).then(|| {
        let (content, after) = rest.split_at(length);
        (tag, content, after)
    })
}

/// When a certificate expires and who issued it. shortcut: reads these two fields of the TBS by hand, since the
/// workspace holds no X.509 parser; take one if a checker comes to need more of a certificate.
fn facts(certificate: &[u8]) -> (Option<String>, Option<String>) {
    fn read(certificate: &[u8]) -> Option<(Option<String>, Option<String>)> {
        let (_, whole, _) = element(certificate)?;
        let (_, tbs, _) = element(whole)?;
        let (tag, _, after) = element(tbs)?;
        // An optional `[0]` version, then the serial; then the signature algorithm.
        let rest = if tag == 0xa0 {
            element(after)?.2
        } else {
            after
        };
        let (_, _, rest) = element(rest)?;
        let (_, issuer, rest) = element(rest)?;
        let (_, validity, _) = element(rest)?;
        let (_, _, after_start) = element(validity)?;
        let (tag, time, _) = element(after_start)?;
        Some((rfc3339(tag, time), common_name(issuer)))
    }
    read(certificate).unwrap_or((None, None))
}

/// A UTCTime (tag 0x17, `YYMMDDHHMMSSZ`) or GeneralizedTime (0x18, `YYYYMMDDHHMMSSZ`) as `2030-01-31T23:59:59Z`.
fn rfc3339(tag: u8, time: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(time).ok()?;
    let (century, rest) = match tag {
        0x17 => {
            let year: u32 = text.get(..2)?.parse().ok()?;
            (if year >= 50 { "19" } else { "20" }, text)
        }
        0x18 => (text.get(..2)?, text.get(2..)?),
        _ => return None,
    };
    if rest.len() != 13 || !rest.ends_with('Z') || !rest[..12].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!(
        "{century}{}-{}-{}T{}:{}:{}Z",
        &rest[..2],
        &rest[2..4],
        &rest[4..6],
        &rest[6..8],
        &rest[8..10],
        &rest[10..12]
    ))
}

/// The common name in an X.501 name.
fn common_name(mut name: &[u8]) -> Option<String> {
    const COMMON_NAME: [u8; 3] = [0x55, 0x04, 0x03];
    while !name.is_empty() {
        let (_, set, rest) = element(name)?;
        let (_, pair, _) = element(set)?;
        let (_, oid, value) = element(pair)?;
        if oid == COMMON_NAME {
            let (_, text, _) = element(value)?;
            return Some(String::from_utf8_lossy(text).into_owned());
        }
        name = rest;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_certificates_expiry_and_issuer_are_read() {
        let (not_after, issuer) = facts(include_bytes!("../tests/self-signed.cert.der"));
        assert_eq!(not_after.as_deref(), Some("2126-10-10T14:47:58Z"));
        assert_eq!(issuer.as_deref(), Some("localhost"));
    }

    #[test]
    fn not_a_certificate_reads_as_nothing() {
        assert_eq!(facts(b"nonsense"), (None, None));
        assert_eq!(facts(&[]), (None, None));
    }

    #[test]
    fn both_time_forms_read_and_a_bad_one_does_not() {
        assert_eq!(
            rfc3339(0x17, b"300131235959Z").as_deref(),
            Some("2030-01-31T23:59:59Z")
        );
        assert_eq!(
            rfc3339(0x17, b"991231235959Z").as_deref(),
            Some("1999-12-31T23:59:59Z")
        );
        assert_eq!(
            rfc3339(0x18, b"21261010144758Z").as_deref(),
            Some("2126-10-10T14:47:58Z")
        );
        assert_eq!(rfc3339(0x18, b"2126"), None);
        assert_eq!(rfc3339(0x02, b"300131235959Z"), None);
    }
}
