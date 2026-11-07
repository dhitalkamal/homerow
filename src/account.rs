// github login, badge sync and logout. github is reached through Http and the
// token through Secrets (the os keychain in real runs), so the whole flow runs
// on fakes in tests. the link file only says which gist holds the badge.
use crate::brand::NAME;
use crate::github::{self, ApiError, Reply, Request};
use crate::history::{self, View};
use crate::store::Progress;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub trait Http {
    fn send(&mut self, req: &Request) -> Result<Reply, String>;
}

// where the token lives; a missing token is Ok(None), not an error
pub trait Secrets {
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, token: &str) -> Result<(), String>;
    // removing a token that is not there is fine
    fn delete(&self) -> Result<(), String>;
}

// what login needs from the terminal
pub trait LoginUi {
    fn say(&mut self, line: &str);
    // show the login page; the real one waits for enter, then opens a browser
    fn open(&mut self, url: &str);
    fn sleep(&mut self, secs: u64);
}

// which gist holds your badge. not secret: the token is in the keychain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub user: String,
    pub gist: String,
    // kept per link so a later rename of the app does not break old badges
    pub file: String,
    // the badge text last pushed, so an unchanged badge is not sent again
    #[serde(default)]
    pub pushed: String,
}

#[derive(Debug, PartialEq)]
pub enum Synced {
    Pushed(String),
    Unchanged,
}

#[derive(Debug, PartialEq)]
pub enum SyncError {
    NotLinked,
    NoToken,
    Keychain(String),
    // github refused the saved token (revoked or expired)
    Expired,
    // the badge gist was deleted
    Gone,
    Network(String),
    Github(String),
}

impl SyncError {
    // worth a line even after a quiet sync on quit: only you can fix these
    pub fn loud(&self) -> bool {
        matches!(
            self,
            SyncError::NoToken | SyncError::Expired | SyncError::Gone
        )
    }
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SyncError::NotLinked => write!(f, "not signed in, run: {} login", NAME),
            SyncError::NoToken => write!(f, "no saved github login, run: {} login", NAME),
            SyncError::Keychain(e) => write!(f, "could not read the keychain: {e}"),
            SyncError::Expired => write!(
                f,
                "github no longer accepts the saved login, run: {} login",
                NAME
            ),
            SyncError::Gone => write!(
                f,
                "your badge gist is gone, run: {} login (makes a new one)",
                NAME
            ),
            SyncError::Network(e) => write!(f, "could not reach github: {e}"),
            SyncError::Github(e) => write!(f, "{e}"),
        }
    }
}

pub fn badge_file() -> String {
    format!("{NAME}-badge.json")
}

// the link file sits next to progress.json
pub fn link_path(progress: &Path) -> PathBuf {
    progress.with_file_name("github.json")
}

pub fn load_link(path: &Path) -> Option<Link> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

// temp file then rename, same as progress, so a crash never leaves half a file
pub fn save_link(path: &Path, link: &Link) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(link)?)?;
    fs::rename(&tmp, path)
}

// the badge number: average of your last 3 baseline rounds
pub fn badge_wpm(p: &Progress) -> Option<u32> {
    history::change(p, View::Base).map(|c| c.now)
}

pub fn sync(
    http: &mut dyn Http,
    secrets: &dyn Secrets,
    link_path: &Path,
    wpm: Option<u32>,
) -> Result<Synced, SyncError> {
    let mut link = load_link(link_path).ok_or(SyncError::NotLinked)?;
    let message = github::badge_message(wpm);
    if message == link.pushed {
        return Ok(Synced::Unchanged);
    }
    let token = secrets
        .get()
        .map_err(SyncError::Keychain)?
        .ok_or(SyncError::NoToken)?;
    let req = github::update_gist_request(&token, &link.gist, &link.file, &github::badge_json(wpm));
    let reply = http.send(&req).map_err(SyncError::Network)?;
    match github::read_gist(&reply) {
        Ok(_) => {}
        Err(ApiError::Unauthorized) => return Err(SyncError::Expired),
        Err(ApiError::NotFound) => return Err(SyncError::Gone),
        Err(ApiError::Other(m)) => return Err(SyncError::Github(m)),
    }
    link.pushed = message.clone();
    save_link(link_path, &link).map_err(|e| SyncError::Github(e.to_string()))?;
    Ok(Synced::Pushed(message))
}

// forget the token and the link; the gist itself stays on github
pub fn logout(secrets: &dyn Secrets, link_path: &Path) -> Result<String, String> {
    secrets
        .delete()
        .map_err(|e| format!("could not remove the login from your keychain: {e}"))?;
    let Some(link) = load_link(link_path) else {
        return Ok("not signed in to github on this computer".to_string());
    };
    fs::remove_file(link_path)
        .map_err(|e| format!("could not remove {}: {e}", link_path.display()))?;
    Ok(format!(
        "signed out of github on this computer.\n\
         your badge gist stays at {} (delete it there if you like).\n\
         to remove {NAME}'s access completely: https://github.com/settings/applications",
        github::gist_page(&link.user, &link.gist)
    ))
}

mod login;
pub use login::login;

#[cfg(test)]
#[path = "account/fakes.rs"]
mod fakes;
#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
