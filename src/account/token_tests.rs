use super::*;
use crate::account::fakes::*;
use crate::account::load_link;
use crate::github::Reply;

const CLASSIC: &str = "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FINE: &str = "github_pat_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const USER: &str = r#"{"login":"octocat"}"#;

fn answer(status: u16, body: &str, scopes: Option<&str>) -> Result<Reply, String> {
    Ok(Reply {
        status,
        body: body.into(),
        scopes: scopes.map(str::to_string),
    })
}

fn http(answers: Vec<Result<Reply, String>>) -> FakeHttp {
    FakeHttp {
        replies: answers.into(),
        sent: Vec::new(),
    }
}

#[test]
fn a_gist_only_token_signs_in_without_a_client_id() {
    let (_t, p) = temp_link();
    let mut h = http(vec![
        answer(200, USER, Some("gist")),
        answer(201, GIST, None),
    ]);
    let s = FakeSecrets::default();
    let mut ui = FakeUi::default();
    let l = login_with_token(&mut h, &s, &mut ui, CLASSIC, &p, Some(52)).unwrap();
    assert_eq!((l.user.as_str(), l.gist.as_str()), ("octocat", "abc123"));
    assert_eq!(s.token.borrow().as_deref(), Some(CLASSIC));
    assert_eq!(load_link(&p), Some(l));
    assert_eq!(h.sent[0].url, "https://api.github.com/user");
    assert_eq!(h.sent[0].token.as_deref(), Some(CLASSIC));
    assert!(
        ui.opened.is_empty() && ui.slept.is_empty(),
        "no device flow"
    );
    assert!(
        ui.said.iter().any(|l| l.contains("img.shields.io")),
        "{:?}",
        ui.said
    );
}

#[test]
fn a_broad_token_is_refused_before_anything_is_made() {
    let (_t, p) = temp_link();
    let mut h = http(vec![answer(200, USER, Some("gist, repo, delete_repo"))]);
    let s = FakeSecrets::default();
    let e = login_with_token(&mut h, &s, &mut FakeUi::default(), CLASSIC, &p, None).unwrap_err();
    assert!(e.contains("repo, delete_repo"), "{e}");
    assert!(e.contains(&github::token_page()), "{e}");
    assert_eq!(h.sent.len(), 1, "no gist made");
    assert_eq!(*s.token.borrow(), None);
    assert_eq!(load_link(&p), None);
}

#[test]
fn a_token_without_the_gist_scope_is_refused() {
    let (_t, p) = temp_link();
    let mut h = http(vec![answer(200, USER, Some("read:user"))]);
    let e = login_with_token(
        &mut h,
        &FakeSecrets::default(),
        &mut FakeUi::default(),
        CLASSIC,
        &p,
        None,
    )
    .unwrap_err();
    assert!(e.contains("gist scope"), "{e}");
    assert_eq!(h.sent.len(), 1);
}

#[test]
fn a_fine_grained_token_gets_a_reminder() {
    let (_t, p) = temp_link();
    let mut h = http(vec![answer(200, USER, None), answer(201, GIST, None)]);
    let mut ui = FakeUi::default();
    login_with_token(&mut h, &FakeSecrets::default(), &mut ui, FINE, &p, None).unwrap();
    assert!(ui.said.iter().any(|l| l.contains("Gists")), "{:?}", ui.said);
}

#[test]
fn a_fine_grained_token_without_gist_access_explains_why() {
    let (_t, p) = temp_link();
    let refused = r#"{"message":"Resource not accessible by personal access token"}"#;
    let mut h = http(vec![answer(200, USER, None), answer(403, refused, None)]);
    let s = FakeSecrets::default();
    let e = login_with_token(&mut h, &s, &mut FakeUi::default(), FINE, &p, None).unwrap_err();
    assert!(e.contains("Resource not accessible"), "{e}");
    assert!(e.contains("Gists"), "{e}");
    assert_eq!(*s.token.borrow(), None);
}

#[test]
fn a_rejected_or_malformed_token_stores_nothing() {
    let (_t, p) = temp_link();
    let mut h = http(vec![answer(401, r#"{"message":"Bad credentials"}"#, None)]);
    let s = FakeSecrets::default();
    let e = login_with_token(&mut h, &s, &mut FakeUi::default(), CLASSIC, &p, None).unwrap_err();
    assert!(e.contains("did not accept"), "{e}");
    let mut none = FakeHttp::default();
    let e = login_with_token(
        &mut none,
        &s,
        &mut FakeUi::default(),
        "not a token",
        &p,
        None,
    )
    .unwrap_err();
    assert!(e.contains("does not look like"), "{e}");
    assert!(none.sent.is_empty(), "a bad token is never sent");
    assert_eq!(*s.token.borrow(), None);
}
