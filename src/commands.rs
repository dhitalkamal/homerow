// the github commands (login, logout, sync) and the quiet badge sync when the
// trainer quits. the real work is in account.rs; this connects it to the
// terminal, the network and the keychain.
use crate::account::{self, LoginUi, SyncError, Synced};
use crate::brand::{CLIENT_ID_ENV, GITHUB_CLIENT_ID};
use crate::cli::Command;
use crate::github::badge_message;
use crate::keychain::Keychain;
use crate::net::Net;
use crate::store::{self, Progress};
use std::io::BufRead;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

// login waits on you, so it gets longer than the sync on quit
const LOGIN_TIMEOUT: u64 = 15;
const QUIT_TIMEOUT: u64 = 4;

// the env var wins, so a build without a client id can still be tried
pub fn client_id(env: Option<String>) -> String {
    env.map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| GITHUB_CLIENT_ID.to_string())
}

// what to print after a sync. on quit it only speaks up for a new badge or
// for a problem only you can fix; offline stays silent and retries next time.
pub fn sync_message(r: &Result<Synced, SyncError>, now: &str, quiet: bool) -> Option<String> {
    match r {
        Ok(Synced::Pushed(m)) => Some(format!("github badge updated: typing {m}")),
        Ok(Synced::Unchanged) if quiet => None,
        Ok(Synced::Unchanged) => Some(format!("github badge already shows typing {now}")),
        Err(e) if quiet && !e.loud() => None,
        Err(e) => Some(format!("github badge: {e}")),
    }
}

// runs one github command; the result is the process exit code
pub fn run(cmd: Command) -> i32 {
    let progress = store::default_path();
    let (p, note) = store::load(&progress);
    if let Some(n) = note {
        eprintln!("{n}");
    }
    let link = account::link_path(&progress);
    let wpm = account::badge_wpm(&p);
    let done = match cmd {
        Command::Login => login(&link, wpm),
        Command::Sync => sync(&link, wpm),
        Command::Logout => logout(&link),
    };
    match done {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

// after the trainer closes: push the badge if you are signed in. people who
// never signed in never touch the keychain or the network.
pub fn sync_on_quit(progress: &Path, p: &Progress) {
    let link = account::link_path(progress);
    if account::load_link(&link).is_none() {
        return;
    }
    let Ok(keychain) = Keychain::new() else {
        return;
    };
    let wpm = account::badge_wpm(p);
    let r = account::sync(&mut Net::new(QUIT_TIMEOUT), &keychain, &link, wpm);
    if let Some(m) = sync_message(&r, &badge_message(wpm), true) {
        println!("{m}");
    }
}

fn keychain() -> Result<Keychain, String> {
    Keychain::new().map_err(|e| format!("could not open the keychain: {e}"))
}

fn login(link: &Path, wpm: Option<u32>) -> Result<(), String> {
    let id = client_id(std::env::var(CLIENT_ID_ENV).ok());
    let mut net = Net::new(LOGIN_TIMEOUT);
    account::login(&mut net, &keychain()?, &mut Terminal, &id, link, wpm).map(|_| ())
}

fn sync(link: &Path, wpm: Option<u32>) -> Result<(), String> {
    let r = account::sync(&mut Net::new(LOGIN_TIMEOUT), &keychain()?, link, wpm);
    let msg = sync_message(&r, &badge_message(wpm), false).unwrap_or_default();
    match r {
        Ok(_) => {
            println!("{msg}");
            Ok(())
        }
        Err(_) => Err(msg),
    }
}

fn logout(link: &Path) -> Result<(), String> {
    println!("{}", account::logout(&keychain()?, link)?);
    Ok(())
}

// login talks to you on the plain terminal (the trainer screen is not open)
struct Terminal;

impl LoginUi for Terminal {
    fn say(&mut self, line: &str) {
        println!("{line}");
    }

    fn open(&mut self, url: &str) {
        println!("then press Enter to open {url} in your browser");
        let mut line = String::new();
        // nothing to read (input piped or closed): the page is printed above
        if std::io::stdin().lock().read_line(&mut line).unwrap_or(0) > 0 {
            open_browser(url);
        }
        println!("waiting for you to approve it on github (ctrl+c stops)");
    }

    fn sleep(&mut self, secs: u64) {
        std::thread::sleep(Duration::from_secs(secs));
    }
}

// best effort, the page is printed anyway. no shell: the url is one argument
// to the system's opener, and account.rs only passes github.com pages.
fn open_browser(url: &str) {
    let (cmd, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(windows) {
        ("rundll32", &["url.dll,FileProtocolHandler"])
    } else {
        ("xdg-open", &[])
    };
    let _ = std::process::Command::new(cmd)
        .args(args)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_client_id_wins_when_set() {
        assert_eq!(client_id(Some(" Ov23env ".into())), "Ov23env");
        assert_eq!(client_id(Some("  ".into())), GITHUB_CLIENT_ID);
        assert_eq!(client_id(None), GITHUB_CLIENT_ID);
    }

    #[test]
    fn sync_on_demand_always_answers() {
        let pushed = Ok(Synced::Pushed("52 wpm".into()));
        assert_eq!(
            sync_message(&pushed, "52 wpm", false).unwrap(),
            "github badge updated: typing 52 wpm"
        );
        let same = sync_message(&Ok(Synced::Unchanged), "52 wpm", false).unwrap();
        assert!(
            same.contains("already") && same.contains("52 wpm"),
            "{same}"
        );
        let off = sync_message(&Err(SyncError::Network("offline".into())), "", false).unwrap();
        assert!(off.contains("offline"), "{off}");
        assert!(sync_message(&Err(SyncError::NotLinked), "", false).is_some());
    }

    #[test]
    fn sync_on_quit_stays_quiet_unless_it_matters() {
        let q = |r: Result<Synced, SyncError>| sync_message(&r, "52 wpm", true);
        assert!(q(Ok(Synced::Pushed("52 wpm".into()))).is_some());
        assert_eq!(q(Ok(Synced::Unchanged)), None);
        assert_eq!(q(Err(SyncError::NotLinked)), None);
        assert_eq!(q(Err(SyncError::Network("offline".into()))), None);
        assert_eq!(q(Err(SyncError::Keychain("denied".into()))), None);
        assert!(q(Err(SyncError::Expired)).unwrap().contains("login"));
        assert!(q(Err(SyncError::Gone)).is_some());
    }
}
