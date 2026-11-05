//! A loopback HTTP server for the tests that reach a real socket: the Http adapter's, the registry clients' and the
//! `specforge http` verb's. Each connection's request is read (its head through `httparse`, then as many body bytes
//! as its `Content-Length` says), kept, and answered by the test's closure, which writes to the stream itself, so it
//! may send a slow, chunked or broken answer as well as a plain one ([`response`]).

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A request the server read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    /// The request target, as sent: `/v1/models?x=1`.
    pub path: String,
    /// The headers, names as sent.
    pub headers: Vec<(String, String)>,
    /// The head as received, up to its blank line.
    pub head: String,
    pub body: Vec<u8>,
}

impl Request {
    /// The value of the header `name` (in any case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The head, then the body as text: what a test that matches the whole request reads.
    pub fn text(&self) -> String {
        format!("{}{}", self.head, String::from_utf8_lossy(&self.body))
    }
}

/// A server on `127.0.0.1` that answers every connection on its own thread, in order.
pub struct Server {
    /// `http://127.0.0.1:<port>`.
    pub base: String,
    seen: Arc<Mutex<Vec<Request>>>,
    arrived: Mutex<Receiver<Request>>,
}

impl Server {
    /// A server whose `answer` is handed each request, numbered from 0, and the stream to write its answer to. A
    /// request whose head does not parse is kept as far as it was read and answered all the same.
    pub fn serve(answer: impl Fn(usize, &Request, &mut TcpStream) + Send + 'static) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is free");
        let base = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("a bound listener").port()
        );
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        let (tell, arrived) = channel();
        std::thread::spawn(move || {
            for (n, stream) in listener.incoming().enumerate() {
                let Ok(mut stream) = stream else { continue };
                let request = read(&mut stream);
                log.lock()
                    .expect("no test panics holding it")
                    .push(request.clone());
                let _ = tell.send(request.clone());
                answer(n, &request, &mut stream);
            }
        });
        Server {
            base,
            seen,
            arrived: Mutex::new(arrived),
        }
    }

    /// A server that answers each request, numbered from 0, with the bytes `answer` gives.
    pub fn reply<B: AsRef<[u8]>>(answer: impl Fn(usize, &Request) -> B + Send + 'static) -> Server {
        Self::serve(move |n, request, stream| {
            let _ = stream.write_all(answer(n, request).as_ref());
        })
    }

    /// A server that answers every request `status` with a `content_type` `body`.
    pub fn answering(status: u16, content_type: &str, body: &str) -> Server {
        let answer = response(status, &[("Content-Type", content_type)], body.as_bytes());
        Self::reply(move |_, _| answer.clone())
    }

    /// Each request it got so far, in order.
    pub fn requests(&self) -> Vec<Request> {
        self.seen.lock().expect("no test panics holding it").clone()
    }

    /// The next request not yet taken, once it arrives within `within`.
    pub fn next(&self, within: Duration) -> Option<Request> {
        self.arrived
            .lock()
            .expect("no test panics holding it")
            .recv_timeout(within)
            .ok()
    }

    /// A port on this machine nothing listens on.
    pub fn closed_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is free");
        listener.local_addr().expect("a bound listener").port()
    }
}

/// An HTTP/1.1 answer of `status` with `headers`, its `Content-Length` and `Connection: close`, then `body`.
pub fn response(status: u16, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut head = format!("HTTP/1.1 {status} X\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// The request on `stream`: its head read up to the blank line and parsed, then its body. A body is read before the
/// answer, since answering a request unread resets the connection.
fn read(stream: &mut TcpStream) -> Request {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
        head.push(byte[0]);
    }
    let mut parsed = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut parsed);
    let _ = request.parse(&head);
    let headers: Vec<(String, String)> = request
        .headers
        .iter()
        .take_while(|header| !header.name.is_empty())
        .map(|header| {
            (
                header.name.to_string(),
                String::from_utf8_lossy(header.value).into_owned(),
            )
        })
        .collect();
    let mut read = Request {
        method: request.method.unwrap_or_default().to_string(),
        path: request.path.unwrap_or_default().to_string(),
        headers,
        head: String::from_utf8_lossy(&head).into_owned(),
        body: Vec::new(),
    };
    let length = read
        .header("content-length")
        .and_then(|length| length.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    if stream.read_exact(&mut body).is_ok() {
        read.body = body;
    }
    read
}
