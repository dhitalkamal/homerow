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
            test: None,
        },
        60_000,
    );
    a.progress.add_round(
        Round {
            mode: "code".into(),
            wpm: 22,
            acc: 97,
            at_ms: NOW - 120_000,
            test: None,
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
            test: None,
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

#[test]
fn code_tab_names_the_file_you_are_typing() {
    let (mut a, _d) = app();
    a.own = vec!["let total = add(1, 2);".to_string()];
    a.own_name = Some("main.rs".to_string());
    go(&mut a, Mode::Code);
    let s = screen(&a, 100, 40);
    assert!(s.contains("from main.rs"), "{s}");
}

#[test]
fn stats_line_shows_the_test_and_counts_down_from_its_length() {
    let (mut a, _d) = app();
    a.progress.settings.test = crate::baseline::Length::Time(30);
    a.handle(Input::Tab, NOW);
    a.handle(Input::BackTab, NOW);
    let s = screen(&a, 100, 40);
    assert!(s.contains("30s left") && s.contains("30s"), "{s}");
}

#[test]
fn results_compare_only_with_the_same_test() {
    let (mut a, _d) = app();
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 99,
            acc: 98,
            at_ms: NOW - 100_000,
            test: Some("60s".into()),
        },
        60_000,
    );
    a.progress.settings.test = crate::baseline::Length::Words(25);
    a.handle(Input::Tab, NOW);
    a.handle(Input::BackTab, NOW);
    let text: Vec<char> = a.session.text.clone();
    let mut t = NOW - 60_000;
    for ch in text {
        if a.session.next_char() == Some(ch) {
            a.handle(Input::Text(ch), t);
            t += 150;
        }
    }
    let s = screen(&a, 100, 40);
    assert!(
        !s.contains("best 99"),
        "a 60s best shown on a 25 word result\n{s}"
    );
}

#[test]
fn history_shows_which_test_each_baseline_round_ran() {
    let (mut a, _d) = app();
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 44,
            acc: 97,
            at_ms: NOW - 60_000,
            test: Some("30s +punct".into()),
        },
        60_000,
    );
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 38,
            acc: 95,
            at_ms: NOW - 90_000,
            test: None,
        },
        60_000,
    );
    go(&mut a, Mode::Progress);
    a.handle(Input::Text('\n'), NOW);
    let s = screen(&a, 110, 40);
    assert!(
        s.contains("baseline 30s +punct") && s.contains("baseline 60s"),
        "{s}"
    );
}

#[test]
fn ghost_shows_on_the_text_and_the_stats_line() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let (mut a, _d) = app();
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 40,
            acc: 95,
            at_ms: 0,
            test: None,
        },
        60_000,
    );
    a.handle(Input::Tab, NOW);
    a.handle(Input::BackTab, NOW);
    let first = a.session.next_char().unwrap();
    a.handle(Input::Text(first), NOW - 6_000);
    a.tick(NOW);
    let s = screen(&a, 100, 40);
    assert!(s.contains("ghost 40"), "{s}");
    assert!(
        s.contains("-19") || s.contains("- 19"),
        "1 typed, ghost at 20\n{s}"
    );
    let mut runs = Vec::new();
    let mut t = Terminal::new(TestBackend::new(100, 40)).unwrap();
    t.draw(|f| crate::ui::draw(f, &a, NOW, &mut runs)).unwrap();
    let buf = t.backend().buffer();
    let ghosted = buf
        .content()
        .iter()
        .filter(|c| c.bg == crate::ui::KEYCAP)
        .count();
    assert_eq!(ghosted, 1, "exactly one ghost letter (keyboard is off)");
}

#[test]
fn ghost_also_shows_in_kitty_zoom() {
    let (mut a, _d) = app();
    a.zoom_ok = true;
    a.progress.settings.zoom = 2;
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 40,
            acc: 95,
            at_ms: 0,
            test: None,
        },
        60_000,
    );
    a.handle(Input::Tab, NOW);
    a.handle(Input::BackTab, NOW);
    let first = a.session.next_char().unwrap();
    a.handle(Input::Text(first), NOW - 6_000);
    a.tick(NOW);
    let (_, runs) = super::tests::screen_and_runs(&a, 150, 40);
    assert!(
        runs.iter().any(|r| r.style.bg == Some(crate::ui::KEYCAP)),
        "no ghost in zoomed text"
    );
}

#[test]
fn slow_pairs_show_on_the_drill_and_the_dashboard() {
    let (mut a, _d) = app();
    a.progress.pairs.insert(
        "ng".into(),
        crate::stats::KeyStat {
            n: 20,
            e: 8,
            ms: 12_000,
        },
    );
    a.progress.add_round(
        Round {
            mode: "keys".into(),
            wpm: 30,
            acc: 90,
            at_ms: NOW,
            test: None,
        },
        60_000,
    );
    go(&mut a, Mode::Keys);
    assert!(
        screen(&a, 110, 40).contains("ng"),
        "focus line shows the pair"
    );
    go(&mut a, Mode::Progress);
    let s = screen(&a, 120, 40);
    assert!(s.contains("slow pairs") && s.contains("ng"), "{s}");
}

#[test]
fn lessons_show_the_keyboard_and_what_is_next() {
    let (mut a, _d) = app();
    assert!(!a.progress.settings.keyboard);
    go(&mut a, Mode::Lessons);
    let s = screen(&a, 100, 44);
    assert!(s.contains("shift"), "keyboard shows in lessons\n{s}");
    assert!(s.contains("lesson 8/27") && s.contains("new key"), "{s}");
    let text: Vec<char> = a.session.text.clone();
    let mut t = NOW - 30_000;
    for ch in text {
        if a.session.next_char() == Some(ch) {
            a.handle(Input::Text(ch), t);
            t += 100;
        }
    }
    let s = screen(&a, 100, 44);
    assert!(s.contains("unlocked G"), "{s}");
}
