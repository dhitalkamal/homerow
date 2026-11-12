use super::*;
use crate::account::fakes::*;
use crate::account::{Link, badge_file, load_link, save_link};
use crate::github::{self, Reply};
use std::path::Path;

fn run_login(
    http: &mut FakeHttp,
    s: &FakeSecrets,
    ui: &mut FakeUi,
    path: &Path,
) -> Result<Link, String> {
    login(http, s, ui, "Ov23id", path, Some(52))
}

#[test]
fn login_needs_a_client_id() {
    let (_t, p) = temp_link();
    let mut http = FakeHttp::default();
    let e = login(
        &mut http,
        &FakeSecrets::default(),
        &mut FakeUi::default(),
        "",
        &p,
        None,
    )
    .unwrap_err();
    assert!(e.contains("not set up"), "{e}");
    assert!(e.contains("login --token"), "{e}");
    assert!(http.sent.is_empty());
}

#[test]
fn login_shows_the_code_polls_and_makes_the_badge_gist() {
    let (_t, p) = temp_link();
    let mut http = FakeHttp::with(&[
        (200, DEVICE),
        (200, PENDING),
        (200, r#"{"error":"slow_down","interval":10}"#),
        (200, TOKEN),
        (201, GIST),
    ]);
    let s = FakeSecrets::default();
    let mut ui = FakeUi::default();
    let l = run_login(&mut http, &s, &mut ui, &p).unwrap();
    assert!(
        ui.said.iter().any(|l| l.contains("WDJB-MJHT")),
        "{:?}",
        ui.said
    );
    assert_eq!(ui.opened, vec!["https://github.com/login/device"]);
    assert_eq!(ui.slept, vec![5, 5, 10], "waits, and slows down when told");
    assert_eq!(s.token.borrow().as_deref(), Some("gho_tok"));
    assert_eq!(
        l,
        Link {
            pushed: "52 wpm".into(),
            ..link("")
        }
    );
    assert_eq!(load_link(&p), Some(l));
    let last = http.sent.last().unwrap();
    assert_eq!(
        (last.method, last.url.as_str()),
        ("POST", "https://api.github.com/gists")
    );
    assert_eq!(last.token.as_deref(), Some("gho_tok"));
    let readme = github::readme_line("octocat", "abc123", &badge_file());
    assert!(ui.said.contains(&readme), "{:?}", ui.said);
}

#[test]
fn slow_down_without_an_interval_adds_five_seconds() {
    let (_t, p) = temp_link();
    let mut http = FakeHttp::with(&[
        (200, DEVICE),
        (200, r#"{"error":"slow_down"}"#),
        (200, TOKEN),
        (201, GIST),
    ]);
    let mut ui = FakeUi::default();
    run_login(&mut http, &FakeSecrets::default(), &mut ui, &p).unwrap();
    assert_eq!(ui.slept, vec![5, 10]);
}

#[test]
fn login_again_reuses_your_badge_gist() {
    let (_t, p) = temp_link();
    save_link(&p, &link("40 wpm")).unwrap();
    let mut http = FakeHttp::with(&[(200, DEVICE), (200, TOKEN), (200, GIST)]);
    let l = run_login(
        &mut http,
        &FakeSecrets::default(),
        &mut FakeUi::default(),
        &p,
    )
    .unwrap();
    let last = http.sent.last().unwrap();
    assert_eq!(
        (last.method, last.url.as_str()),
        ("PATCH", "https://api.github.com/gists/abc123")
    );
    assert_eq!(l.gist, "abc123");
    assert_eq!(l.pushed, "52 wpm");
}

#[test]
fn login_makes_a_new_gist_when_the_old_one_is_gone() {
    let (_t, p) = temp_link();
    save_link(&p, &link("40 wpm")).unwrap();
    let mut http = FakeHttp::with(&[
        (200, DEVICE),
        (200, TOKEN),
        (404, r#"{"message":"Not Found"}"#),
        (201, r#"{"id":"new456","owner":{"login":"octocat"}}"#),
    ]);
    let l = run_login(
        &mut http,
        &FakeSecrets::default(),
        &mut FakeUi::default(),
        &p,
    )
    .unwrap();
    assert_eq!(l.gist, "new456");
    assert_eq!(http.sent.last().unwrap().method, "POST");
}

#[test]
fn login_with_another_account_makes_its_own_gist() {
    // the old gist is readable by url but not yours to edit
    let (_t, p) = temp_link();
    save_link(&p, &link("40 wpm")).unwrap();
    let mut http = FakeHttp::with(&[
        (200, DEVICE),
        (200, TOKEN),
        (403, r#"{"message":"Forbidden"}"#),
        (201, r#"{"id":"mine789","owner":{"login":"hubot"}}"#),
    ]);
    let l = run_login(
        &mut http,
        &FakeSecrets::default(),
        &mut FakeUi::default(),
        &p,
    )
    .unwrap();
    assert_eq!((l.user.as_str(), l.gist.as_str()), ("hubot", "mine789"));
}

#[test]
fn denied_or_expired_logins_store_nothing() {
    for (answer, word) in [
        (r#"{"error":"access_denied"}"#, "cancelled"),
        (r#"{"error":"expired_token"}"#, "expired"),
    ] {
        let (_t, p) = temp_link();
        let mut http = FakeHttp::with(&[(200, DEVICE), (200, answer)]);
        let s = FakeSecrets::default();
        let e = run_login(&mut http, &s, &mut FakeUi::default(), &p).unwrap_err();
        assert!(e.contains(word), "{e}");
        assert_eq!(*s.token.borrow(), None);
        assert_eq!(load_link(&p), None);
    }
}

#[test]
fn login_gives_up_when_the_code_runs_out() {
    let (_t, p) = temp_link();
    let short = DEVICE.replace("\"expires_in\":900", "\"expires_in\":10");
    let mut http = FakeHttp::with(&[(200, &short), (200, PENDING), (200, PENDING)]);
    let mut ui = FakeUi::default();
    let e = run_login(&mut http, &FakeSecrets::default(), &mut ui, &p).unwrap_err();
    assert!(e.contains("expired"), "{e}");
    assert_eq!(http.sent.len(), 3, "no poll after the code ran out");
}

#[test]
fn an_unexpected_login_page_is_never_opened() {
    let (_t, p) = temp_link();
    let odd = DEVICE.replace(
        "https://github.com/login/device",
        "https://evil.example/login",
    );
    let mut http = FakeHttp::with(&[(200, &odd)]);
    let mut ui = FakeUi::default();
    assert!(run_login(&mut http, &FakeSecrets::default(), &mut ui, &p).is_err());
    assert!(ui.opened.is_empty());
}

#[test]
fn a_network_blip_while_polling_is_tolerated() {
    let (_t, p) = temp_link();
    let mut http = FakeHttp::with(&[(200, DEVICE)]);
    http.replies.push_back(Err("timed out".into()));
    for (s, b) in [(200, TOKEN), (201, GIST)] {
        http.replies.push_back(Ok(Reply {
            status: s,
            body: b.into(),
            ..Default::default()
        }));
    }
    assert!(
        run_login(
            &mut http,
            &FakeSecrets::default(),
            &mut FakeUi::default(),
            &p
        )
        .is_ok()
    );
}

#[test]
fn a_keychain_failure_is_reported_but_the_gist_is_remembered() {
    let (_t, p) = temp_link();
    let mut http = FakeHttp::with(&[(200, DEVICE), (200, TOKEN), (201, GIST)]);
    let s = FakeSecrets {
        broken: true,
        ..Default::default()
    };
    let e = run_login(&mut http, &s, &mut FakeUi::default(), &p).unwrap_err();
    assert!(e.contains("keychain"), "{e}");
    assert_eq!(load_link(&p).unwrap().gist, "abc123", "a retry reuses it");
}
