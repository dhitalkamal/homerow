// the real Http: ureq with a short timeout, github's errors returned as
// replies (not errors) so account.rs can read them, and a plain user agent
use crate::account::Http;
use crate::brand::NAME;
use crate::github::{Reply, Request};
use std::time::Duration;

pub struct Net {
    agent: ureq::Agent,
}

impl Net {
    pub fn new(timeout_secs: u64) -> Net {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(timeout_secs)))
            .http_status_as_error(false)
            .user_agent(NAME)
            .build()
            .into();
        Net { agent }
    }
}

impl Http for Net {
    fn send(&mut self, req: &Request) -> Result<Reply, String> {
        let auth = req.token.as_ref().map(|t| format!("Bearer {t}"));
        let sent = match (req.method, &req.body) {
            ("GET", None) => {
                let mut b = self.agent.get(&req.url).header("Accept", req.accept);
                if let Some(a) = &auth {
                    b = b.header("Authorization", a);
                }
                b.call()
            }
            (method, Some((kind, body))) => {
                let mut b = match method {
                    "POST" => self.agent.post(&req.url),
                    "PATCH" => self.agent.patch(&req.url),
                    other => return Err(format!("unsupported method {other}")),
                }
                .header("Accept", req.accept)
                .header("Content-Type", *kind);
                if let Some(a) = &auth {
                    b = b.header("Authorization", a);
                }
                b.send(body.as_str())
            }
            (other, None) => return Err(format!("unsupported method {other} without a body")),
        };
        let mut resp = sent.map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        Ok(Reply { status, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    // answers one request with the given status line and body, and hands
    // back the raw request it got
    fn serve_once(status: &'static str, body: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            // headers first, then as much body as content-length says
            loop {
                let n = s.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_lowercase();
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if got.len() >= end + 4 + len || n == 0 {
                        break;
                    }
                }
            }
            let reply = format!(
                "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            s.write_all(reply.as_bytes()).unwrap();
            tx.send(String::from_utf8_lossy(&got).into_owned()).unwrap();
        });
        (format!("http://{addr}"), rx)
    }

    #[test]
    fn sends_method_headers_and_body() {
        let (base, got) = serve_once("200 OK", r#"{"ok":1}"#);
        let req = Request {
            method: "PATCH",
            url: format!("{base}/gists/x"),
            accept: "application/vnd.github+json",
            token: Some("tok".into()),
            body: Some(("application/json", r#"{"a":1}"#.into())),
        };
        let r = Net::new(5).send(&req).unwrap();
        assert_eq!(
            r,
            Reply {
                status: 200,
                body: r#"{"ok":1}"#.into()
            }
        );
        let raw = got.recv().unwrap();
        let low = raw.to_lowercase();
        assert!(low.starts_with("patch /gists/x http/1.1"), "{raw}");
        assert!(low.contains("authorization: bearer tok"), "{raw}");
        assert!(low.contains(&format!("user-agent: {NAME}\r\n")), "{raw}");
        assert!(low.contains("accept: application/vnd.github+json"), "{raw}");
        assert!(low.contains("content-type: application/json"), "{raw}");
        assert!(raw.ends_with(r#"{"a":1}"#), "{raw}");
    }

    #[test]
    fn error_statuses_come_back_as_replies() {
        let (base, _got) = serve_once("401 Unauthorized", r#"{"message":"Bad credentials"}"#);
        let req = Request {
            method: "GET",
            url: format!("{base}/user"),
            accept: "application/json",
            token: None,
            body: None,
        };
        let r = Net::new(5).send(&req).unwrap();
        assert_eq!(r.status, 401);
        assert!(r.body.contains("Bad credentials"));
    }

    #[test]
    fn login_posts_carry_no_token() {
        let (base, got) = serve_once("200 OK", "{}");
        let req = Request {
            method: "POST",
            url: format!("{base}/login/device/code"),
            accept: "application/json",
            token: None,
            body: Some(("application/x-www-form-urlencoded", "client_id=x".into())),
        };
        Net::new(5).send(&req).unwrap();
        let low = got.recv().unwrap().to_lowercase();
        assert!(!low.contains("authorization"), "{low}");
        assert!(
            low.contains("content-type: application/x-www-form-urlencoded"),
            "{low}"
        );
    }

    #[test]
    fn an_unreachable_server_is_an_error() {
        // grab a free port, then close it so nothing listens there
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let req = Request {
            method: "GET",
            url: format!("http://127.0.0.1:{port}/"),
            accept: "application/json",
            token: None,
            body: None,
        };
        assert!(Net::new(2).send(&req).is_err());
    }
}
