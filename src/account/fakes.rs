// fakes shared by the account and login tests: canned github answers, an
// in-memory keychain and a terminal that only records what it was told
use super::*;
use crate::github::{Reply, Request};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

// replays canned answers and remembers every request
#[derive(Default)]
pub(super) struct FakeHttp {
    pub(super) replies: VecDeque<Result<Reply, String>>,
    pub(super) sent: Vec<Request>,
}

impl FakeHttp {
    pub(super) fn with(replies: &[(u16, &str)]) -> Self {
        FakeHttp {
            replies: replies
                .iter()
                .map(|(s, b)| {
                    Ok(Reply {
                        status: *s,
                        body: b.to_string(),
                        ..Default::default()
                    })
                })
                .collect(),
            sent: Vec::new(),
        }
    }
}

impl Http for FakeHttp {
    fn send(&mut self, req: &Request) -> Result<Reply, String> {
        self.sent.push(req.clone());
        self.replies
            .pop_front()
            .unwrap_or(Err("no more replies".into()))
    }
}

#[derive(Default)]
pub(super) struct FakeSecrets {
    pub(super) token: RefCell<Option<String>>,
    pub(super) gets: Cell<u32>,
    pub(super) broken: bool,
}

impl Secrets for FakeSecrets {
    fn get(&self) -> Result<Option<String>, String> {
        self.gets.set(self.gets.get() + 1);
        Ok(self.token.borrow().clone())
    }
    fn set(&self, token: &str) -> Result<(), String> {
        if self.broken {
            return Err("keychain locked".into());
        }
        *self.token.borrow_mut() = Some(token.into());
        Ok(())
    }
    fn delete(&self) -> Result<(), String> {
        if self.broken {
            return Err("keychain locked".into());
        }
        *self.token.borrow_mut() = None;
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct FakeUi {
    pub(super) said: Vec<String>,
    pub(super) opened: Vec<String>,
    pub(super) slept: Vec<u64>,
}

impl LoginUi for FakeUi {
    fn say(&mut self, line: &str) {
        self.said.push(line.into());
    }
    fn open(&mut self, url: &str) {
        self.opened.push(url.into());
    }
    fn sleep(&mut self, secs: u64) {
        self.slept.push(secs);
    }
}

pub(super) const DEVICE: &str = r#"{"device_code":"dev1","user_code":"WDJB-MJHT","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#;
pub(super) const PENDING: &str = r#"{"error":"authorization_pending"}"#;
pub(super) const TOKEN: &str = r#"{"access_token":"gho_tok","token_type":"bearer","scope":"gist"}"#;
pub(super) const GIST: &str = r#"{"id":"abc123","owner":{"login":"octocat"}}"#;

pub(super) fn temp_link() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("github.json");
    (t, p)
}

pub(super) fn link(pushed: &str) -> Link {
    Link {
        user: "octocat".into(),
        gist: "abc123".into(),
        file: badge_file(),
        pushed: pushed.into(),
    }
}

pub(super) fn signed_in(pushed: &str) -> (tempfile::TempDir, PathBuf, FakeSecrets) {
    let (t, p) = temp_link();
    save_link(&p, &link(pushed)).unwrap();
    let s = FakeSecrets::default();
    *s.token.borrow_mut() = Some("gho_tok".into());
    (t, p, s)
}
