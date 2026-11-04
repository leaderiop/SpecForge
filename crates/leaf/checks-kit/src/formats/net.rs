//! Addresses and names: RFC 5321, RFC 791, RFC 4291, RFC 4632, RFC 1123, RFC 6335 and CSS Color 4.

use std::net::IpAddr;

use email_address::{EmailAddress, Options};

use crate::Invalid;

/// An RFC 5321 addr-spec: no display name, and a domain that names a host.
pub fn email(text: &str) -> Result<(), Invalid> {
    let options = Options {
        minimum_sub_domains: 2,
        allow_domain_literal: false,
        allow_display_text: false,
    };
    EmailAddress::parse_with_options(text, options)
        .map(|_| ())
        .map_err(|error| Invalid::new(error.to_string()))
}

/// An IP address, of `version` 4 or 6 when one is named.
pub fn ip(text: &str, version: Option<u8>) -> Result<(), Invalid> {
    let address: IpAddr = text
        .parse()
        .map_err(|_| Invalid::new("this is not an IP address"))?;
    ip_version(&address, version)
}

fn ip_version(address: &IpAddr, version: Option<u8>) -> Result<(), Invalid> {
    match (address, version) {
        (IpAddr::V4(_), Some(6)) => Err(Invalid::new("this is an IPv4 address, not IPv6")),
        (IpAddr::V6(_), Some(4)) => Err(Invalid::new("this is an IPv6 address, not IPv4")),
        _ => Ok(()),
    }
}

/// A network as address, `/` and prefix length; when `strict`, with its host bits zero.
pub fn cidr(text: &str, strict: bool) -> Result<(), Invalid> {
    let Some((address, length)) = text.split_once('/') else {
        return Err(Invalid::new("a network ends in `/` and a prefix length"));
    };
    let parsed: IpAddr = address
        .parse()
        .map_err(|_| Invalid::at("this is not an IP address", 0..address.len()))?;
    let digits = address.len() + 1..text.len();
    let bits = match parsed {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    let prefix: u32 = (!length.is_empty() && length.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| length.parse().ok())
        .flatten()
        .filter(|prefix| *prefix <= bits)
        .ok_or_else(|| Invalid::at(format!("a prefix length is 0 to {bits}"), digits.clone()))?;
    let host_mask = match bits - prefix {
        0 => 0,
        host => u128::MAX >> (128 - host),
    };
    let value = match parsed {
        IpAddr::V4(address) => u128::from(u32::from(address)),
        IpAddr::V6(address) => u128::from(address),
    };
    if strict && value & host_mask != 0 {
        return Err(Invalid::at(
            format!("the bits after /{prefix} are not zero"),
            0..address.len(),
        ));
    }
    Ok(())
}

/// A host name of RFC 1123, in A-labels. `wildcard` allows a leading `*.`.
pub fn hostname(text: &str, wildcard: bool) -> Result<(), Invalid> {
    let (from, name) = match text.strip_prefix("*.") {
        Some(rest) if wildcard => (2, rest),
        _ => (0, text),
    };
    if name.is_empty() || name.len() > 253 {
        return Err(Invalid::new("a host name has 1 to 253 characters"));
    }
    let mut at = from;
    let mut last = "";
    for label in name.split('.') {
        let span = at..at + label.len();
        if label.is_empty() || label.len() > 63 {
            return Err(Invalid::at("a label has 1 to 63 characters", span));
        }
        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(Invalid::at(
                "a label holds letters, digits and hyphens; write another script as its A-label",
                span,
            ));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(Invalid::at(
                "a label neither starts nor ends with a hyphen",
                span,
            ));
        }
        at += label.len() + 1;
        last = label;
    }
    // RFC 1123 2.1: the last label is not all digits, so an address is never a name.
    if last.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Invalid::new("a host name's last label is not all digits"));
    }
    Ok(())
}

/// A port number, 1 to 65535; `zero` allows 0.
pub fn port(text: &str, zero: bool) -> Result<(), Invalid> {
    let digits = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    match text.parse::<u16>() {
        Ok(0) if digits && !zero => Err(Invalid::new("port 0 is not allowed here")),
        Ok(_) if digits => Ok(()),
        _ => Err(Invalid::new("a port is a number from 0 to 65535")),
    }
}

/// A CSS hex color: `#` then 3, 4, 6 or 8 hexadecimal digits.
pub fn hex_color(text: &str) -> Result<(), Invalid> {
    let Some(digits) = text.strip_prefix('#') else {
        return Err(Invalid::new("a hex color starts with `#`"));
    };
    if let Some(at) = digits.find(|ch: char| !ch.is_ascii_hexdigit()) {
        return Err(Invalid::char_at(
            "this is not a hexadecimal digit",
            text,
            at + 1,
        ));
    }
    match digits.len() {
        3 | 4 | 6 | 8 => Ok(()),
        other => Err(Invalid::new(format!(
            "a hex color has 3, 4, 6 or 8 digits, this one has {other}"
        ))),
    }
}
