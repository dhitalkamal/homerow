// pasted personal access tokens: is it shaped like a token, and may it do
// more than edit gists. classic tokens report their scopes in a header,
// fine-grained ones do not.
use super::{ACCEPT_API, API, ApiError, Reply, Request, api_error, plain};
use crate::brand::NAME;
use serde_json::Value;

// what a pasted token may do, judged by the scopes github reports for it
#[derive(Debug, PartialEq)]
pub enum Scopes {
    GistOnly,
    // fine-grained tokens report nothing, so they can only be trusted
    Unknown,
    // a classic token without the gist scope
    Missing,
    // a classic token that can do more than gists: the extra scopes
    Broad(Vec<String>),
}

// a pasted token goes into a request header: plain characters only
pub fn token_ok(token: &str) -> bool {
    (20..=255).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_graphic())
}

pub fn scope_check(token: &str, header: Option<&str>) -> Scopes {
    // fine-grained tokens start like this and carry permissions, not scopes
    if token.starts_with("github_pat_") {
        return Scopes::Unknown;
    }
    let Some(header) = header else {
        return Scopes::Unknown;
    };
    let scopes: Vec<&str> = header
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if !scopes.contains(&"gist") {
        return Scopes::Missing;
    }
    let extra: Vec<String> = scopes
        .iter()
        .filter(|s| **s != "gist")
        .map(|s| s.to_string())
        .collect();
    if extra.is_empty() {
        Scopes::GistOnly
    } else {
        Scopes::Broad(extra)
    }
}

pub fn user_request(token: &str) -> Request {
    Request {
        method: "GET",
        url: format!("{API}/user"),
        accept: ACCEPT_API,
        token: Some(token.to_string()),
        body: None,
    }
}

pub fn read_user(r: &Reply) -> Result<String, ApiError> {
    if !(200..300).contains(&r.status) {
        return Err(api_error(r));
    }
    let v: Value = serde_json::from_str(&r.body)
        .map_err(|_| ApiError::Other("github sent an unreadable user".to_string()))?;
    match v["login"].as_str() {
        Some(login) if plain(login) => Ok(login.to_string()),
        _ => Err(ApiError::Other("github sent an odd user name".to_string())),
    }
}

// github's new classic token page with only the gist scope ticked
pub fn token_page() -> String {
    format!("https://github.com/settings/tokens/new?scopes=gist&description={NAME}")
}
