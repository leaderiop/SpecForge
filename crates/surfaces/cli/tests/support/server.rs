//! A one-route HTTP server on `127.0.0.1` for the verbs that reach the network (`specforge http`): every request gets
//! the same answer, and each request's head and body are kept.

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

pub struct Server {
    /// `http://127.0.0.1:<port>`.
    pub base: String,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Server {
    /// A server that answers `status` with a `content_type` `body` to every request.
    pub fn answering(status: u16, content_type: &str, body: &str) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        let answer = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                    head.push(byte[0]);
                }
                let mut request = String::from_utf8_lossy(&head).into_owned();
                let length = request
                    .lines()
                    .find_map(|l| {
                        let (name, value) = l.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                let mut body = vec![0u8; length];
                let _ = stream.read_exact(&mut body);
                request.push_str(&String::from_utf8_lossy(&body));
                log.lock().unwrap().push(request);
                let _ = stream.write_all(answer.as_bytes());
            }
        });
        Server { base, seen }
    }

    /// Each request it got: its head, then its body.
    pub fn requests(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }

    /// A port on this machine nothing listens on.
    pub fn closed_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }
}
