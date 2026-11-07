// screen tests for the mistakes tab and round history
use super::tests::{NOW, app, go, screen};
use crate::app::{Input, Mode};
use crate::store::Round;

#[test]
fn mistakes_tab_explains_itself_when_there_are_none() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Mistakes);
    let s = screen(&a, 100, 40);
    assert!(s.contains("No mistakes to practice yet"), "{s}");
}

#[test]
fn history_view_lists_rounds_newest_first() {
    let (mut a, _d) = app();
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 31,
            acc: 94,
            at_ms: NOW - 3 * 3_600_000,
        },
        60_000,
    );
    a.progress.add_round(
        Round {
            mode: "code".into(),
            wpm: 22,
            acc: 97,
            at_ms: NOW - 120_000,
        },
        60_000,
    );
    go(&mut a, Mode::Progress);
    a.handle(Input::Text('\n'), NOW);
    let s = screen(&a, 110, 40);
    let newer = s.find("2m ago").expect("newest round");
    let older = s.find("3h ago").expect("older round");
    assert!(newer < older, "newest first\n{s}");
    assert!(s.contains("31") && s.contains("94%"), "{s}");
}

#[test]
fn progress_shows_todays_minutes_against_the_goal() {
    let (mut a, _d) = app();
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 30,
            acc: 95,
            at_ms: NOW,
        },
        12 * 60_000,
    );
    go(&mut a, Mode::Progress);
    let s = screen(&a, 120, 40);
    assert!(s.contains("12/15"), "{s}");
}

#[test]
fn results_screen_shows_the_daily_goal() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Code);
    let text: Vec<char> = a.session.text.clone();
    let mut t = NOW - 60_000;
    for ch in text {
        if a.session.next_char() == Some(ch) {
            a.handle(Input::Text(ch), t);
            t += 150;
        }
    }
    assert!(a.session.is_done());
    let s = screen(&a, 100, 40);
    assert!(s.contains("/15 min today"), "{s}");
}
