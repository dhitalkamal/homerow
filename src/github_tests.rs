use super::*;

fn reply(status: u16, body: &str) -> Reply {
    Reply {
        status,
        body: body.to_string(),
        ..Default::default()
    }
}

#[test]
fn urlencode_keeps_only_unreserved() {
    assert_eq!(urlencode("aZ09-._~"), "aZ09-._~");
    assert_eq!(
        urlencode("https://x.com/a b?c=d&e"),
        "https%3A%2F%2Fx.com%2Fa%20b%3Fc%3Dd%26e"
    );
    assert_eq!(urlencode(":"), "%3A");
}

#[test]
fn device_code_request_asks_for_gist_scope_only() {
    let r = device_code_request("Ov23abc");
    assert_eq!(r.method, "POST");
    assert_eq!(r.url, DEVICE_CODE_URL);
    assert_eq!(r.accept, "application/json");
    assert_eq!(r.token, None);
    assert_eq!(
        r.body,
        Some((
            "application/x-www-form-urlencoded",
            "client_id=Ov23abc&scope=gist".to_string()
        ))
    );
}

#[test]
fn reads_a_device_code() {
    let body = r#"{"device_code":"3584d83","user_code":"WDJB-MJHT",
        "verification_uri":"https://github.com/login/device",
        "expires_in":900,"interval":5}"#;
    let d = read_device_code(&reply(200, body)).unwrap();
    assert_eq!(d.device_code, "3584d83");
    assert_eq!(d.user_code, "WDJB-MJHT");
    assert_eq!(d.verification_uri, "https://github.com/login/device");
    assert_eq!((d.expires_in, d.interval), (900, 5));
}

#[test]
fn device_code_errors_are_explained() {
    // github answers 200 with an error body for a bad client id or a disabled flow
    let body = r#"{"error":"device_flow_disabled","error_description":"Device Flow must be explicitly enabled for this App"}"#;
    let e = read_device_code(&reply(200, body)).unwrap_err();
    assert!(e.contains("Device Flow must be explicitly enabled"), "{e}");
    assert!(read_device_code(&reply(500, "oops")).is_err());
    assert!(read_device_code(&reply(200, "not json")).is_err());
}

#[test]
fn error_bodies_are_read_whatever_the_status() {
    let body = r#"{"error":"Not Found","error_description":"check the client id"}"#;
    let e = read_device_code(&reply(404, body)).unwrap_err();
    assert!(e.contains("check the client id"), "{e}");
    let e = read_device_code(&reply(404, "<html>")).unwrap_err();
    assert!(e.contains("404"), "{e}");
}

#[test]
fn a_user_code_with_odd_characters_is_refused() {
    // it is printed to your terminal, so it may not carry escape codes
    let body = r#"{"device_code":"x","user_code":"AB\u001b[2J",
        "verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#;
    assert!(read_device_code(&reply(200, body)).is_err());
}

#[test]
fn token_request_uses_the_device_grant() {
    let r = token_request("Ov23abc", "dev123");
    assert_eq!(r.url, TOKEN_URL);
    assert_eq!(r.token, None);
    let (kind, body) = r.body.unwrap();
    assert_eq!(kind, "application/x-www-form-urlencoded");
    assert_eq!(
        body,
        "client_id=Ov23abc&device_code=dev123&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"
    );
}

#[test]
fn reads_every_poll_answer() {
    let t = |b: &str| read_token(&reply(200, b));
    assert_eq!(
        t(r#"{"access_token":"gho_abc","token_type":"bearer","scope":"gist"}"#),
        Poll::Token("gho_abc".into())
    );
    assert_eq!(t(r#"{"error":"authorization_pending"}"#), Poll::Pending);
    assert_eq!(
        t(r#"{"error":"slow_down","interval":10}"#),
        Poll::SlowDown(10)
    );
    assert_eq!(t(r#"{"error":"expired_token"}"#), Poll::Expired);
    assert_eq!(t(r#"{"error":"access_denied"}"#), Poll::Denied);
    match t(r#"{"error":"incorrect_client_credentials","error_description":"bad id"}"#) {
        Poll::Failed(m) => assert!(m.contains("bad id"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(read_token(&reply(502, "")), Poll::Failed(_)));
}

#[test]
fn slow_down_without_an_interval_adds_five_seconds_later() {
    // 0 means "add the usual 5", the caller knows the current interval
    assert_eq!(
        read_token(&reply(200, r#"{"error":"slow_down"}"#)),
        Poll::SlowDown(0)
    );
}

#[test]
fn badge_says_recent_speed() {
    assert_eq!(badge_message(Some(52)), "52 wpm");
    assert_eq!(badge_message(None), "no rounds yet");
    let v: Value = serde_json::from_str(&badge_json(Some(52))).unwrap();
    assert_eq!(v["schemaVersion"], 1);
    assert_eq!(v["label"], "typing");
    assert_eq!(v["message"], "52 wpm");
    assert_eq!(v["color"], "blue");
    let none: Value = serde_json::from_str(&badge_json(None)).unwrap();
    assert_eq!(none["color"], "lightgrey");
}

#[test]
fn creates_a_secret_gist_with_the_badge_file() {
    let r = create_gist_request("tok", "homerow-badge.json", "{}");
    assert_eq!(r.method, "POST");
    assert_eq!(r.url, "https://api.github.com/gists");
    assert_eq!(r.accept, "application/vnd.github+json");
    assert_eq!(r.token.as_deref(), Some("tok"));
    let (kind, body) = r.body.unwrap();
    assert_eq!(kind, "application/json");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["public"], false);
    assert_eq!(v["files"]["homerow-badge.json"]["content"], "{}");
}

#[test]
fn updates_only_the_badge_file() {
    let r = update_gist_request("tok", "abc123", "homerow-badge.json", "{\"a\":1}");
    assert_eq!(r.method, "PATCH");
    assert_eq!(r.url, "https://api.github.com/gists/abc123");
    let v: Value = serde_json::from_str(&r.body.unwrap().1).unwrap();
    assert_eq!(v["files"]["homerow-badge.json"]["content"], "{\"a\":1}");
    assert!(
        v.get("public").is_none(),
        "an update never changes visibility"
    );
}

#[test]
fn reads_the_gist_id_and_owner() {
    let body = r#"{"id":"aa5a315d61ae9438b18d","owner":{"login":"octocat"}}"#;
    assert_eq!(
        read_gist(&reply(201, body)),
        Ok(Gist {
            id: "aa5a315d61ae9438b18d".into(),
            owner: "octocat".into()
        })
    );
    assert_eq!(read_gist(&reply(200, body)).unwrap().owner, "octocat");
}

#[test]
fn gist_ids_and_names_must_be_plain() {
    // both end up in urls and on your terminal
    let bad = r#"{"id":"../x","owner":{"login":"octocat"}}"#;
    assert!(read_gist(&reply(201, bad)).is_err());
    let bad_owner = r#"{"id":"abc","owner":{"login":"a\u001b]0;x"}}"#;
    assert!(read_gist(&reply(201, bad_owner)).is_err());
}

#[test]
fn api_errors_by_status() {
    assert_eq!(api_error(&reply(401, "{}")), ApiError::Unauthorized);
    assert_eq!(api_error(&reply(404, "{}")), ApiError::NotFound);
    match api_error(&reply(422, r#"{"message":"Validation Failed"}"#)) {
        ApiError::Other(m) => assert!(m.contains("422") && m.contains("Validation Failed"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(read_gist(&reply(401, "{}")), Err(ApiError::Unauthorized));
}

#[test]
fn badge_urls_and_readme_line() {
    let raw = raw_url("octocat", "abc123", "homerow-badge.json");
    assert_eq!(
        raw,
        "https://gist.githubusercontent.com/octocat/abc123/raw/homerow-badge.json"
    );
    assert_eq!(
        gist_page("octocat", "abc123"),
        "https://gist.github.com/octocat/abc123"
    );
    let line = readme_line("octocat", "abc123", "homerow-badge.json");
    assert_eq!(
        line,
        format!(
            "[![typing speed](https://img.shields.io/endpoint?url={})](https://{})",
            urlencode(&raw),
            crate::brand::REPO
        )
    );
}

#[test]
fn only_github_pages_are_opened() {
    assert!(safe_to_open("https://github.com/login/device"));
    assert!(!safe_to_open("http://github.com/login/device"));
    assert!(!safe_to_open("https://github.com.evil.example/login"));
    assert!(!safe_to_open("file:///etc/passwd"));
}

#[test]
fn token_format() {
    assert!(token_ok(&format!("ghp_{}", "a".repeat(36))));
    assert!(token_ok(&format!("github_pat_{}", "A1_".repeat(25))));
    assert!(!token_ok("short"));
    assert!(!token_ok(&format!("ghp_{} x", "a".repeat(36))), "no spaces");
    assert!(!token_ok(&format!("ghp_{}\u{1b}", "a".repeat(36))));
    assert!(!token_ok(&"a".repeat(300)));
}

#[test]
fn scopes_of_classic_tokens_are_checked() {
    let classic = format!("ghp_{}", "a".repeat(36));
    assert_eq!(scope_check(&classic, Some("gist")), Scopes::GistOnly);
    assert_eq!(
        scope_check(&classic, Some("gist, repo, delete_repo")),
        Scopes::Broad(vec!["repo".into(), "delete_repo".into()])
    );
    assert_eq!(scope_check(&classic, Some("")), Scopes::Missing);
    assert_eq!(scope_check(&classic, Some("read:user")), Scopes::Missing);
    assert_eq!(scope_check(&classic, None), Scopes::Unknown);
}

#[test]
fn fine_grained_tokens_cannot_be_checked() {
    let fine = format!("github_pat_{}", "A".repeat(70));
    assert_eq!(scope_check(&fine, Some("")), Scopes::Unknown);
    assert_eq!(scope_check(&fine, None), Scopes::Unknown);
}

#[test]
fn user_request_and_reply() {
    let r = user_request("tok");
    assert_eq!(
        (r.method, r.url.as_str()),
        ("GET", "https://api.github.com/user")
    );
    assert_eq!(r.accept, "application/vnd.github+json");
    assert_eq!(r.token.as_deref(), Some("tok"));
    assert_eq!(r.body, None);
    assert_eq!(
        read_user(&reply(200, r#"{"login":"octocat"}"#)),
        Ok("octocat".into())
    );
    assert_eq!(read_user(&reply(401, "{}")), Err(ApiError::Unauthorized));
    assert!(read_user(&reply(200, r#"{"login":"a b"}"#)).is_err());
}

#[test]
fn token_page_makes_a_gist_only_classic_token() {
    assert_eq!(
        token_page(),
        format!(
            "https://github.com/settings/tokens/new?scopes=gist&description={}",
            crate::brand::NAME
        )
    );
}
