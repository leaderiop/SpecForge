//! What the OS's entropy gives ops: bytes, and bytes as lowercase hex.

/// `N` bytes of the OS's entropy.
pub(crate) fn random_bytes<const N: usize>() -> Result<[u8; N], getrandom::Error> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes)?;
    Ok(bytes)
}

/// `bytes` as lowercase hex, two digits each.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `N` bytes of the OS's entropy as lowercase hex.
pub(crate) fn random_hex<const N: usize>() -> Result<String, getrandom::Error> {
    random_bytes::<N>().map(|bytes| hex(&bytes))
}
