// the device flow login: show the code, wait for your approval on github,
// then make (or reuse) the badge gist and keep the token in the keychain
use super::{Http, Link, LoginUi, Secrets, badge_file, load_link, save_link};
use crate::brand::NAME;
use crate::github::{self, ApiError, Gist, Poll, Scopes};
use std::path::Path;

// a few dropped polls in a row are a dead connection, not a blip
const MAX_POLL_ERRORS: u32 = 3;

pub fn login(
    http: &mut dyn Http,
    secrets: &dyn Secrets,
    ui: &mut dyn LoginUi,
    client_id: &str,
    link_path: &Path,
    wpm: Option<u32>,
) -> Result<Link, String> {
    if client_id.is_empty() {
        return Err(format!(
            "github login is not set up in this build yet, use: {NAME} login --token"
        ));
    }
    let reply = http.send(&github::device_code_request(client_id))?;
    let code = github::read_device_code(&reply)?;
    if !github::safe_to_open(&code.verification_uri) {
        return Err("github sent an unexpected login page, stopping".to_string());
    }
    ui.say(&format!(
        "first copy your one-time code: {}",
        code.user_code
    ));
    ui.open(&code.verification_uri);
    let token = poll(http, ui, client_id, &code)?;

    finish(http, secrets, ui, &token, link_path, wpm)
}

// the part both logins share: make or reuse the badge gist, remember it,
// keep the token in the keychain and print the readme line
fn finish(
    http: &mut dyn Http,
    secrets: &dyn Secrets,
    ui: &mut dyn LoginUi,
    token: &str,
    link_path: &Path,
    wpm: Option<u32>,
) -> Result<Link, String> {
    let file = load_link(link_path).map_or_else(badge_file, |l| l.file);
    let content = github::badge_json(wpm);
    let gist = badge_gist(http, token, link_path, &file, &content)?;
    let link = Link {
        user: gist.owner,
        gist: gist.id,
        file,
        pushed: github::badge_message(wpm),
    };
    // remembered before the token is stored, so a keychain failure still
    // lets the next login reuse this gist
    save_link(link_path, &link)
        .map_err(|e| format!("could not save {}: {e}", link_path.display()))?;
    secrets
        .set(token)
        .map_err(|e| format!("could not save the login in your keychain: {e}"))?;
    ui.say(&format!(
        "signed in as {}, badge: typing {}",
        link.user, link.pushed
    ));
    ui.say(&format!(
        "add this line to your profile readme (github.com/{0}/{0}):",
        link.user
    ));
    ui.say(&github::readme_line(&link.user, &link.gist, &link.file));
    ui.say(&format!(
        "it updates when you quit {NAME}, or run: {NAME} sync"
    ));
    Ok(link)
}

// wait for you to approve the code on github, as fast as github allows
fn poll(
    http: &mut dyn Http,
    ui: &mut dyn LoginUi,
    client_id: &str,
    code: &github::DeviceCode,
) -> Result<String, String> {
    let mut interval = code.interval.max(1);
    let mut waited = 0;
    let mut errors = 0;
    loop {
        if waited + interval > code.expires_in {
            return Err(format!("the code expired, run: {NAME} login"));
        }
        ui.sleep(interval);
        waited += interval;
        let reply = match http.send(&github::token_request(client_id, &code.device_code)) {
            Ok(r) => r,
            Err(e) => {
                errors += 1;
                if errors >= MAX_POLL_ERRORS {
                    return Err(format!("could not reach github: {e}"));
                }
                continue;
            }
        };
        errors = 0;
        match github::read_token(&reply) {
            Poll::Token(t) => return Ok(t),
            Poll::Pending => {}
            Poll::SlowDown(0) => interval += 5,
            Poll::SlowDown(n) => interval = n,
            Poll::Expired => return Err(format!("the code expired, run: {NAME} login")),
            Poll::Denied => return Err("login cancelled on github".to_string()),
            Poll::Failed(e) => return Err(e),
        }
    }
}

// update the gist you already have, or make one when there is none. a
// deleted gist answers 404 and another account's gist 403 or 404; any of
// those (and other api refusals) fall through to a fresh gist, which reports
// the real problem if github is refusing everything.
fn badge_gist(
    http: &mut dyn Http,
    token: &str,
    link_path: &Path,
    file: &str,
    content: &str,
) -> Result<Gist, String> {
    if let Some(old) = load_link(link_path) {
        let reply = http.send(&github::update_gist_request(
            token, &old.gist, file, content,
        ))?;
        match github::read_gist(&reply) {
            Ok(g) => return Ok(g),
            Err(ApiError::Unauthorized) => return Err(api_message(ApiError::Unauthorized)),
            Err(_) => {}
        }
    }
    let reply = http.send(&github::create_gist_request(token, file, content))?;
    github::read_gist(&reply).map_err(api_message)
}

fn api_message(e: ApiError) -> String {
    match e {
        ApiError::Unauthorized => "github refused the new login".to_string(),
        ApiError::NotFound => "github could not find the gist".to_string(),
        ApiError::Other(m) => m,
    }
}

// login with a token you made on github and pasted in, no client id needed
pub fn login_with_token(
    http: &mut dyn Http,
    secrets: &dyn Secrets,
    ui: &mut dyn LoginUi,
    token: &str,
    link_path: &Path,
    wpm: Option<u32>,
) -> Result<Link, String> {
    let token = token.trim();
    if !github::token_ok(token) {
        return Err("that does not look like a github token".to_string());
    }
    let reply = http.send(&github::user_request(token))?;
    github::read_user(&reply).map_err(|e| match e {
        ApiError::Unauthorized => "github did not accept that token".to_string(),
        other => api_message(other),
    })?;
    let page = github::token_page();
    let fine_grained = match github::scope_check(token, reply.scopes.as_deref()) {
        Scopes::GistOnly => false,
        Scopes::Unknown => true,
        Scopes::Missing => {
            return Err(format!(
                "that token has no gist scope. make one with only gist ticked: {page}"
            ));
        }
        Scopes::Broad(extra) => {
            return Err(format!(
                "that token can do more than {NAME} needs ({}). \
                 make one with only gist ticked: {page}",
                extra.join(", ")
            ));
        }
    };
    if fine_grained {
        ui.say("fine-grained token: give it only the Gists permission (read and write)");
    }
    finish(http, secrets, ui, token, link_path, wpm).map_err(|e| {
        if fine_grained {
            format!("{e}. a fine-grained token needs the Gists permission set to read and write")
        } else {
            e
        }
    })
}

#[cfg(test)]
#[path = "login_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "token_tests.rs"]
mod token_tests;
