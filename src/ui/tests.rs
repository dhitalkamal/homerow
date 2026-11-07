// screen smoke tests: render into an in-memory terminal and read the text back
use super::*;
use crate::app::{Input, Mode};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

const NOW: u64 = 1_794_009_600_000 + 15 * 3_600_000;

fn screen(app: &App, w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| draw(f, app, NOW)).unwrap();
    let buf = t.backend().buffer();
    let mut s = String::new();
    for y in 0..h {
        for x in 0..w {
            s.push_str(buf[(x, y)].symbol());
        }
        s.push('\n');
    }
    s
}

fn app() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (App::new(dir.path().join("p.json"), 5), dir)
}

fn first_word(a: &App) -> String {
    a.session.text.iter().take_while(|c| **c != ' ').collect()
}

fn go(a: &mut App, mode: Mode) {
    while a.mode != mode {
        a.handle(Input::Tab, NOW);
    }
}

#[test]
fn typing_screen_is_focused() {
    let (a, _d) = app();
    let s = screen(&a, 100, 40);
    for want in ["baseline", "shortcuts", "progress", "wpm", "shift", "tab"] {
        assert!(s.contains(want), "missing {want:?} in\n{s}");
    }
    assert!(s.contains(&first_word(&a)));
    for clutter in ["Speed trend", "Slowest", "practiced today", "L middle"] {
        assert!(
            !s.contains(clutter),
            "{clutter:?} belongs on the progress tab\n{s}"
        );
    }
}

#[test]
fn stats_line_has_a_blank_line_above_it() {
    let (a, _d) = app();
    let s = screen(&a, 100, 40);
    let rows: Vec<&str> = s.lines().collect();
    let at = rows.iter().position(|r| r.contains("wpm")).unwrap();
    assert!(
        rows[at - 1].trim().is_empty(),
        "text is jammed against stats\n{s}"
    );
}

#[test]
fn next_key_hint_names_the_finger() {
    let (a, _d) = app();
    let s = screen(&a, 100, 40);
    let finger = ["pinky", "ring", "middle", "index", "thumb"]
        .iter()
        .any(|f| s.contains(f));
    assert!(s.contains("next") && finger, "no finger hint in\n{s}");
}

#[test]
fn short_terminal_drops_the_keyboard_but_keeps_the_text() {
    let (a, _d) = app();
    let s = screen(&a, 100, 20);
    assert!(s.contains(&first_word(&a)), "{s}");
    assert!(!s.contains("shift"), "{s}");
}

#[test]
fn results_screen_shows_big_numbers() {
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
    assert!(s.contains(bigtext::BLOCK), "no block digits in\n{s}");
    assert!(s.contains("accuracy") && s.contains("return"), "{s}");
}

#[test]
fn shortcut_screen_shows_the_card() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Cards);
    let s = screen(&a, 100, 40);
    let card = crate::content::CARDS[a.card];
    assert!(s.contains(card.action), "missing action in\n{s}");
    assert!(s.contains("learned"), "{s}");
}

#[test]
fn progress_tab_is_a_dashboard() {
    let (mut a, _d) = app();
    a.progress = crate::demo::progress(NOW);
    go(&mut a, Mode::Progress);
    let s = screen(&a, 110, 40);
    for want in [
        "Speed trend",
        "Weak keys",
        "Decks",
        "day streak",
        "best",
        "Terminal (zsh)",
    ] {
        assert!(s.contains(want), "missing {want:?} in\n{s}");
    }
}

#[test]
fn empty_progress_tab_says_how_to_start() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Progress);
    let s = screen(&a, 100, 40);
    assert!(s.contains("Finish a round"), "{s}");
}

#[test]
fn tiny_terminal_does_not_panic_in_any_mode() {
    let (mut a, _d) = app();
    for _ in 0..Mode::ALL.len() {
        screen(&a, 20, 6);
        screen(&a, 1, 1);
        a.handle(Input::Tab, NOW);
    }
}
