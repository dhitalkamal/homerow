use super::fakes::*;
use super::*;
use crate::store::Round;

#[test]
fn badge_file_follows_the_app_name() {
    assert_eq!(badge_file(), format!("{}-badge.json", crate::brand::NAME));
}

#[test]
fn link_file_sits_next_to_progress_and_round_trips() {
    let (_t, p) = temp_link();
    assert_eq!(
        link_path(Path::new("/x/progress.json")),
        PathBuf::from("/x/github.json")
    );
    assert_eq!(load_link(&p), None);
    save_link(&p, &link("52 wpm")).unwrap();
    assert_eq!(load_link(&p), Some(link("52 wpm")));
    std::fs::write(&p, "garbage").unwrap();
    assert_eq!(load_link(&p), None, "an unreadable link means not linked");
}

#[test]
fn badge_counts_recent_baseline_rounds_only() {
    let mut p = Progress::default();
    assert_eq!(badge_wpm(&p), None);
    for (mode, wpm) in [
        ("base", 30),
        ("code", 10),
        ("base", 40),
        ("base", 50),
        ("base", 60),
    ] {
        p.rounds.push(Round {
            mode: mode.into(),
            wpm,
            acc: 95,
            at_ms: 0,
            test: None,
        });
    }
    assert_eq!(badge_wpm(&p), Some(50), "last 3 baseline rounds: 40 50 60");
}

#[test]
fn sync_pushes_a_changed_badge() {
    let (_t, p, s) = signed_in("40 wpm");
    let mut http = FakeHttp::with(&[(200, GIST)]);
    assert_eq!(
        sync(&mut http, &s, &p, Some(52)),
        Ok(Synced::Pushed("52 wpm".into()))
    );
    let req = &http.sent[0];
    assert_eq!(
        (req.method, req.url.as_str()),
        ("PATCH", "https://api.github.com/gists/abc123")
    );
    assert!(req.body.as_ref().unwrap().1.contains("52 wpm"));
    assert_eq!(load_link(&p).unwrap().pushed, "52 wpm");
}

#[test]
fn an_unchanged_badge_sends_nothing_and_skips_the_keychain() {
    let (_t, p, s) = signed_in("52 wpm");
    let mut http = FakeHttp::default();
    assert_eq!(sync(&mut http, &s, &p, Some(52)), Ok(Synced::Unchanged));
    assert!(http.sent.is_empty());
    assert_eq!(s.gets.get(), 0);
}

#[test]
fn sync_without_login_or_token() {
    let (_t, p) = temp_link();
    let mut http = FakeHttp::default();
    assert_eq!(
        sync(&mut http, &FakeSecrets::default(), &p, Some(52)),
        Err(SyncError::NotLinked)
    );
    save_link(&p, &link("40 wpm")).unwrap();
    let e = sync(&mut http, &FakeSecrets::default(), &p, Some(52)).unwrap_err();
    assert_eq!(e, SyncError::NoToken);
    assert!(e.loud() && e.to_string().contains("keyup login"), "{e}");
}

#[test]
fn sync_failures_keep_the_old_badge_for_a_retry() {
    let cases: [(Result<Reply, String>, SyncError, bool); 3] = [
        (
            Ok(Reply {
                status: 401,
                body: "{}".into(),
                ..Default::default()
            }),
            SyncError::Expired,
            true,
        ),
        (
            Ok(Reply {
                status: 404,
                body: "{}".into(),
                ..Default::default()
            }),
            SyncError::Gone,
            true,
        ),
        (
            Err("offline".into()),
            SyncError::Network("offline".into()),
            false,
        ),
    ];
    for (answer, want, loud) in cases {
        let (_t, p, s) = signed_in("40 wpm");
        let mut http = FakeHttp::default();
        http.replies.push_back(answer);
        let e = sync(&mut http, &s, &p, Some(52)).unwrap_err();
        assert_eq!(e, want);
        assert_eq!(e.loud(), loud, "{e}");
        assert_eq!(load_link(&p).unwrap().pushed, "40 wpm");
    }
}

#[test]
fn logout_forgets_token_and_link_but_points_at_the_gist() {
    let (_t, p, s) = signed_in("52 wpm");
    let msg = logout(&s, &p).unwrap();
    assert_eq!(*s.token.borrow(), None);
    assert_eq!(load_link(&p), None);
    assert!(
        msg.contains(&github::gist_page("octocat", "abc123")),
        "{msg}"
    );
    assert!(msg.contains("github.com/settings/applications"), "{msg}");
}

#[test]
fn logout_when_signed_out_or_keychain_broken() {
    let (_t, p) = temp_link();
    assert!(
        logout(&FakeSecrets::default(), &p)
            .unwrap()
            .contains("not signed in")
    );
    save_link(&p, &link("52 wpm")).unwrap();
    let broken = FakeSecrets {
        broken: true,
        ..Default::default()
    };
    assert!(logout(&broken, &p).unwrap_err().contains("keychain"));
    assert!(load_link(&p).is_some(), "nothing half done");
}
