//! A registry answer is read within a bound (ADR 0250): a download no longer than the `size_bytes` its reply
//! declares (64 MiB when it declares none), any other answer no longer than 64 MiB, all of it within the
//! client's timeout. The registry here is a socket that sends the bodies a real one would not.

use std::io::{Read as _, Write as _};
use std::time::{Duration, Instant};

use specforge_package::{PackageName, Version};
use specforge_registry_client::HttpRegistryClient;
use specforge_registry_client::registry_client::{RegistryClient, RegistryError};
use specforge_registry_client::registry_config::RegistryConfig;
use specforge_registry_wire::RegistryRoot;

const MAX: u64 = 64 * 1024 * 1024;

/// What a fake registry sends for one path.
enum Body {
    /// A body with its `Content-Length`.
    Length(Vec<u8>),
    /// `prefix`, `bytes` copies of `filler`, `suffix`, chunked: one chunk per `every` (a byte per chunk when
    /// `every` is not zero, 64 KiB otherwise).
    Chunked {
        prefix: String,
        bytes: u64,
        suffix: String,
        every: Duration,
    },
}

impl Body {
    fn zeros(bytes: u64) -> Self {
        Body::Chunked {
            prefix: String::new(),
            bytes,
            suffix: String::new(),
            every: Duration::ZERO,
        }
    }

    fn send(&self, stream: &mut std::net::TcpStream) -> std::io::Result<()> {
        match self {
            Body::Length(bytes) => {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                )?;
                stream.write_all(bytes)
            }
            Body::Chunked {
                prefix,
                bytes,
                suffix,
                every,
            } => {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                )?;
                let chunk =
                    |stream: &mut std::net::TcpStream, data: &[u8]| -> std::io::Result<()> {
                        write!(stream, "{:x}\r\n", data.len())?;
                        stream.write_all(data)?;
                        stream.write_all(b"\r\n")?;
                        stream.flush()
                    };
                if !prefix.is_empty() {
                    chunk(stream, prefix.as_bytes())?;
                }
                let step: u64 = if every.is_zero() { 64 * 1024 } else { 1 };
                let filler = if prefix.is_empty() { 0u8 } else { b'a' };
                let mut left = *bytes;
                while left > 0 {
                    let now = step.min(left);
                    chunk(stream, &vec![filler; now as usize])?;
                    left -= now;
                    std::thread::sleep(*every);
                }
                if !suffix.is_empty() {
                    chunk(stream, suffix.as_bytes())?;
                }
                stream.write_all(b"0\r\n\r\n")
            }
        }
    }
}

/// The protocol's metadata for `@acme/tool@1.0.0`, its binary at `/download`, declaring `size` bytes (none when `None`).
fn metadata_json(size: Option<u64>) -> String {
    let size = size.map_or(String::new(), |n| format!(r#""size_bytes":{n},"#));
    format!(
        r#"{{"name":"@acme/tool","version":"1.0.0","sha256":"{}",{size}"wasm_url":"/download"}}"#,
        "0".repeat(64)
    )
}

/// A registry that answers a path ending `/download` with `download` and any other with `metadata`.
fn serve(metadata: Body, download: Body) -> RegistryConfig {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let root = RegistryRoot::parse(&format!(
        "http://127.0.0.1:{}",
        listener.local_addr().unwrap().port()
    ))
    .unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                head.push(byte[0]);
            }
            let head = String::from_utf8_lossy(&head);
            let path = head.split_whitespace().nth(1).unwrap_or("");
            // The client hangs up on a body it refuses: a write error is its answer, not the fixture's.
            let _ = if path.ends_with("/download") {
                download.send(&mut stream)
            } else {
                metadata.send(&mut stream)
            };
        }
    });
    RegistryConfig {
        alias: "slow".into(),
        url: root,
        scope_filter: None,
        default_registry: true,
    }
}

fn the_package() -> (PackageName, Version) {
    (
        PackageName::parse("@acme/tool").unwrap(),
        Version::parse("1.0.0").unwrap(),
    )
}

/// The error of `answer`; its Ok is never printed (it may be 64 MiB).
fn refused<T>(answer: Result<T, RegistryError>) -> RegistryError {
    match answer {
        Err(error) => error,
        Ok(_) => panic!("the answer was read, not refused"),
    }
}

fn r009(error: RegistryError) -> String {
    let diagnostic = error.to_diagnostic();
    assert_eq!(diagnostic.code, "R009", "{diagnostic:?}");
    diagnostic.message
}

#[specforge_test_macros::test(
    behavior = "check_registry_reply",
    verify = "a download larger than the size its reply declares is refused with R009 before it is read whole"
)]
fn a_download_larger_than_its_reply_declares_is_refused_before_it_is_read() {
    let registry = serve(
        Body::Length(metadata_json(Some(10)).into_bytes()),
        Body::Length(vec![0; 1024 * 1024]),
    );
    let client = HttpRegistryClient::new();
    let (name, version) = the_package();
    let metadata = client.metadata(&name, &version, &registry, None).unwrap();
    let error = refused(client.download(&metadata, &registry, None));
    let message = r009(error);
    assert!(
        message.contains("10 bytes") && message.contains("size_bytes"),
        "{message}"
    );
}

#[specforge_test_macros::test(
    behavior = "check_registry_reply",
    verify = "a download larger than the size its reply declares is refused with R009 before it is read whole"
)]
fn a_chunked_download_is_cut_at_the_size_its_reply_declares() {
    let registry = serve(
        Body::Length(metadata_json(Some(10)).into_bytes()),
        Body::zeros(1024 * 1024),
    );
    let client = HttpRegistryClient::new();
    let (name, version) = the_package();
    let metadata = client.metadata(&name, &version, &registry, None).unwrap();
    let error = refused(client.download(&metadata, &registry, None));
    let message = r009(error);
    assert!(
        message.contains("10 bytes") && message.contains("size_bytes"),
        "{message}"
    );
}

#[specforge_test_macros::test(
    behavior = "check_registry_reply",
    verify = "a download whose reply declares no size is held to 64 MiB, the largest package a registry accepts"
)]
fn a_download_whose_reply_declares_no_size_is_held_to_64_mib() {
    let registry = serve(
        Body::Length(metadata_json(None).into_bytes()),
        Body::zeros(MAX + 1),
    );
    let client = HttpRegistryClient::new();
    let (name, version) = the_package();
    let metadata = client.metadata(&name, &version, &registry, None).unwrap();
    let error = refused(client.download(&metadata, &registry, None));
    let message = r009(error);
    assert!(message.contains("64 MiB"), "{message}");
}

#[specforge_test_macros::test(
    behavior = "check_registry_reply",
    verify = "a registry answer larger than 64 MiB is refused with R009"
)]
fn a_registry_answer_larger_than_64_mib_is_refused() {
    let head = metadata_json(None);
    let head = head.strip_suffix('}').unwrap();
    let registry = serve(
        Body::Chunked {
            prefix: format!(r#"{head},"description":""#),
            bytes: MAX + 1,
            suffix: r#""}"#.into(),
            every: Duration::ZERO,
        },
        Body::zeros(0),
    );
    let (name, version) = the_package();
    let error = refused(HttpRegistryClient::new().metadata(&name, &version, &registry, None));
    let message = r009(error);
    assert!(message.contains("64 MiB"), "{message}");
}

#[specforge_test_macros::test(
    behavior = "retry_registry_request",
    verify = "a registry answer still arriving after the client's timeout is R004"
)]
fn a_body_still_arriving_after_the_timeout_is_r004() {
    let registry = serve(
        Body::Length(metadata_json(None).into_bytes()),
        Body::Chunked {
            prefix: String::new(),
            bytes: 40,
            suffix: String::new(),
            every: Duration::from_millis(100),
        },
    );
    let client = HttpRegistryClient::new().with_timeout(Duration::from_secs(1));
    let (name, version) = the_package();
    let metadata = client.metadata(&name, &version, &registry, None).unwrap();
    let started = Instant::now();
    let error = refused(client.download(&metadata, &registry, None));
    assert_eq!(error.to_diagnostic().code, "R004", "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
}
