// github device flow login and the gist that holds the profile badge: request
// builders and reply readers only. nothing here touches the network; account.rs
// sends these through an Http implementation, so all of it is testable.
use crate::brand::REPO;
use serde_json::{Value, json};

pub const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const API: &str = "https://api.github.com";
const GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
// the login endpoints answer form posts with json when asked for json
const ACCEPT_LOGIN: &str = "application/json";
const ACCEPT_API: &str = "application/vnd.github+json";

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: &'static str,
    pub url: String,
    pub accept: &'static str,
    // api calls carry the token, the two login endpoints do not
    pub token: Option<String>,
    // content type and body
    pub body: Option<(&'static str, String)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reply {
    pub status: u16,
    pub body: String,
    // x-oauth-scopes: what a classic token may do. fine-grained tokens and
    // the login endpoints send none.
    pub scopes: Option<String>,
}

// what github hands back to start a device login
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceCode {
    pub device_code: String,
    // the code you type at github.com/login/device
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    // seconds to wait between polls
    pub interval: u64,
}

// one answer while polling for the token
#[derive(Debug, PartialEq)]
pub enum Poll {
    Token(String),
    Pending,
    // poll slower, the new interval in seconds
    SlowDown(u64),
    Expired,
    Denied,
    Failed(String),
}

#[derive(Debug, PartialEq)]
pub struct Gist {
    pub id: String,
    pub owner: String,
}

// api failures that need their own message
#[derive(Debug, PartialEq)]
pub enum ApiError {
    // token revoked or expired
    Unauthorized,
    // gist deleted, or owned by another account
    NotFound,
    Other(String),
}

// percent-encode everything but rfc 3986 unreserved characters
pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn form(pairs: &[(&str, &str)]) -> Option<(&'static str, String)> {
    let body = pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    Some(("application/x-www-form-urlencoded", body))
}

fn api(method: &'static str, path: &str, token: &str, body: Value) -> Request {
    Request {
        method,
        url: format!("{API}{path}"),
        accept: ACCEPT_API,
        token: Some(token.to_string()),
        body: Some(("application/json", body.to_string())),
    }
}

// letters, digits and hyphens only: safe in a url and on a terminal
fn plain(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

// a non-200 answer still has github's explanation in it when it is json
fn parse(r: &Reply) -> Result<Value, String> {
    if r.status != 200 {
        let why = serde_json::from_str::<Value>(&r.body)
            .ok()
            .and_then(|v| oauth_error(&v));
        return Err(why.unwrap_or_else(|| format!("github answered {}", r.status)));
    }
    serde_json::from_str(&r.body).map_err(|_| "github sent an unreadable answer".to_string())
}

// github's own words for an oauth error, falling back to the error code
fn oauth_error(v: &Value) -> Option<String> {
    let code = v["error"].as_str()?;
    Some(v["error_description"].as_str().unwrap_or(code).to_string())
}

pub fn device_code_request(client_id: &str) -> Request {
    Request {
        method: "POST",
        url: DEVICE_CODE_URL.to_string(),
        accept: ACCEPT_LOGIN,
        token: None,
        body: form(&[("client_id", client_id), ("scope", "gist")]),
    }
}

pub fn read_device_code(r: &Reply) -> Result<DeviceCode, String> {
    let v = parse(r)?;
    if let Some(e) = oauth_error(&v) {
        return Err(e);
    }
    let text = |k: &str| v[k].as_str().map(str::to_string);
    let d = DeviceCode {
        device_code: text("device_code").ok_or("no device code in github's answer")?,
        user_code: text("user_code").ok_or("no user code in github's answer")?,
        verification_uri: text("verification_uri").ok_or("no login page in github's answer")?,
        expires_in: v["expires_in"].as_u64().unwrap_or(900),
        interval: v["interval"].as_u64().unwrap_or(5),
    };
    if !plain(&d.user_code) {
        return Err("github sent an odd login code".to_string());
    }
    Ok(d)
}

pub fn token_request(client_id: &str, device_code: &str) -> Request {
    Request {
        method: "POST",
        url: TOKEN_URL.to_string(),
        accept: ACCEPT_LOGIN,
        token: None,
        body: form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", GRANT),
        ]),
    }
}

pub fn read_token(r: &Reply) -> Poll {
    let v = match parse(r) {
        Ok(v) => v,
        Err(e) => return Poll::Failed(e),
    };
    if let Some(t) = v["access_token"].as_str() {
        return Poll::Token(t.to_string());
    }
    match v["error"].as_str() {
        Some("authorization_pending") => Poll::Pending,
        Some("slow_down") => Poll::SlowDown(v["interval"].as_u64().unwrap_or(0)),
        Some("expired_token") => Poll::Expired,
        Some("access_denied") => Poll::Denied,
        _ => Poll::Failed(oauth_error(&v).unwrap_or_else(|| "github sent no token".to_string())),
    }
}

// the badge text: recent baseline speed
pub fn badge_message(wpm: Option<u32>) -> String {
    match wpm {
        Some(w) => format!("{w} wpm"),
        None => "no rounds yet".to_string(),
    }
}

// shields.io endpoint format, see shields.io/badges/endpoint-badge
pub fn badge_json(wpm: Option<u32>) -> String {
    json!({
        "schemaVersion": 1,
        "label": "typing",
        "message": badge_message(wpm),
        "color": if wpm.is_some() { "blue" } else { "lightgrey" },
    })
    .to_string()
}

pub fn create_gist_request(token: &str, file: &str, content: &str) -> Request {
    let body = json!({
        "description": format!("typing badge, kept up to date by {REPO}"),
        "public": false,
        "files": { file: { "content": content } },
    });
    api("POST", "/gists", token, body)
}

pub fn update_gist_request(token: &str, id: &str, file: &str, content: &str) -> Request {
    let body = json!({ "files": { file: { "content": content } } });
    api("PATCH", &format!("/gists/{id}"), token, body)
}

pub fn read_gist(r: &Reply) -> Result<Gist, ApiError> {
    if !(200..300).contains(&r.status) {
        return Err(api_error(r));
    }
    let v: Value = serde_json::from_str(&r.body)
        .map_err(|_| ApiError::Other("github sent an unreadable gist".to_string()))?;
    let id = v["id"].as_str().unwrap_or_default();
    let owner = v["owner"]["login"].as_str().unwrap_or_default();
    if !plain(id) || !plain(owner) {
        return Err(ApiError::Other(
            "github sent an odd gist id or user name".to_string(),
        ));
    }
    Ok(Gist {
        id: id.to_string(),
        owner: owner.to_string(),
    })
}

pub fn api_error(r: &Reply) -> ApiError {
    match r.status {
        401 => ApiError::Unauthorized,
        404 => ApiError::NotFound,
        s => {
            // keep github's message short and printable
            let msg = serde_json::from_str::<Value>(&r.body)
                .ok()
                .and_then(|v| v["message"].as_str().map(str::to_string))
                .unwrap_or_default();
            let msg: String = msg.chars().filter(|c| !c.is_control()).take(120).collect();
            ApiError::Other(format!("github answered {s} {msg}").trim_end().to_string())
        }
    }
}

pub fn raw_url(user: &str, id: &str, file: &str) -> String {
    format!("https://gist.githubusercontent.com/{user}/{id}/raw/{file}")
}

pub fn gist_page(user: &str, id: &str) -> String {
    format!("https://gist.github.com/{user}/{id}")
}

// the line to paste into a profile readme: the badge, linking to the app
pub fn readme_line(user: &str, id: &str, file: &str) -> String {
    format!(
        "[![typing speed](https://img.shields.io/endpoint?url={})](https://{REPO})",
        urlencode(&raw_url(user, id, file))
    )
}

// only github's own pages get opened in a browser
pub fn safe_to_open(url: &str) -> bool {
    url.starts_with("https://github.com/")
}

mod token;
pub use token::{Scopes, read_user, scope_check, token_ok, token_page, user_request};

#[cfg(test)]
#[path = "github_tests.rs"]
mod tests;
