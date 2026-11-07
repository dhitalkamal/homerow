// screen smoke tests: render into an in-memory terminal and read the text back
use super::*;
use crate::app::{Input, Mode};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

const NOW: u64 = 1_794_009_600_000 + 15 * 3_600_000;

fn screen(app: &App, w: u16, h: u16) -> String {
    screen_and_runs(app, w, h).0
}

// the text a test terminal shows, plus the kitty zoom runs drawn on top
fn screen_and_runs(app: &App, w: u16, h: u16) -> (String, Vec<crate::zoom::Run>) {
    let mut runs = Vec::new();
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| draw(f, app, NOW, &mut runs)).unwrap();
    let buf = t.backend().buffer();
    let mut s = String::new();
    for y in 0..h {
        for x in 0..w {
            s.push_str(buf[(x, y)].symbol());
        }
        s.push('\n');
    }
    (s, runs)
}

fn app() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (App::new(dir.path().join("p.json"), 5), dir)
}

fn with_keyboard() -> (App, tempfile::TempDir) {
    let (mut a, d) = app();
    a.progress.settings.keyboard = true;
    (a, d)
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
    for want in [
        "baseline",
        "shortcuts",
        "progress",
        "settings",
        "wpm",
        "tab",
    ] {
        assert!(s.contains(want), "missing {want:?} in\n{s}");
    }
    assert!(s.contains(&first_word(&a)));
    for clutter in [
        "Speed trend",
        "Slowest",
        "practiced today",
        "shift",
        "left hand",
    ] {
        assert!(
            !s.contains(clutter),
            "{clutter:?} belongs on the progress tab\n{s}"
        );
    }
}

#[test]
fn no_hint_text_above_the_typing_area() {
    let (mut a, _d) = app();
    for mode in [Mode::Base, Mode::Keys, Mode::Code] {
        go(&mut a, mode);
        let s = screen(&a, 100, 40);
        let rows: Vec<&str> = s.lines().collect();
        let text_row = rows
            .iter()
            .position(|r| r.contains(&first_word(&a)))
            .unwrap();
        // everything between the tab line and the text must be empty
        assert!(
            rows[3..text_row].iter().all(|r| r.trim().is_empty()),
            "text above typing area in {mode:?}\n{s}"
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
    let (a, _d) = with_keyboard();
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

#[test]
fn size_change_shows_in_the_footer() {
    let (mut a, _d) = app();
    a.handle(Input::Combo(crate::cards::parse("ctrl+=").unwrap()), NOW);
    let s = screen(&a, 100, 40);
    assert!(s.contains("size: spacious"), "{s}");
}

#[test]
fn medium_height_gets_the_compact_keyboard() {
    let (a, _d) = with_keyboard();
    let s = screen(&a, 100, 27);
    assert!(s.contains("shift") && s.contains("next"), "{s}");
}

#[test]
fn keyboard_setting_shows_the_keyboard_and_hands() {
    let (a, _d) = with_keyboard();
    let s = screen(&a, 100, 48);
    for want in ["shift", "next", "left hand", "right hand"] {
        assert!(s.contains(want), "missing {want:?} in\n{s}");
    }
}

#[test]
fn settings_tab_lists_switches_with_a_live_preview() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Settings);
    let s = screen(&a, 100, 48);
    for want in [
        "keyboard",
        "off",
        "hands",
        "size",
        "comfortable",
        "shift",
        "left hand",
    ] {
        assert!(s.contains(want), "missing {want:?} in\n{s}");
    }
}

#[test]
fn auto_size_shows_more_text_on_a_big_terminal() {
    // plain text rows are counted here, so big block letters are switched off
    let (mut a, _d) = app();
    a.progress.settings.big_text = false;
    let rows_of_text = |w, h| {
        let s = screen(&a, w, h);
        let rows: Vec<&str> = s.lines().collect();
        let stats = rows.iter().position(|r| r.contains("wpm")).unwrap();
        rows[3..stats]
            .iter()
            .filter(|r| !r.trim().is_empty())
            .count()
    };
    assert_eq!(rows_of_text(100, 40), 3);
    assert_eq!(rows_of_text(150, 50), 4);
}

#[test]
fn very_big_terminal_gets_the_large_keyboard() {
    let (a, _d) = with_keyboard();
    let big = screen(&a, 190, 52);
    let normal = screen(&a, 100, 48);
    // large keys sit 8 columns apart (6 wide + 2 gap), normal ones 4 apart
    assert!(big.contains("1       2"), "{big}");
    assert!(normal.contains(" 1   2 "), "{normal}");
    assert!(big.contains("left hand"));
}

#[test]
fn footer_avoids_ligature_pairs() {
    // fonts like jetbrains mono draw "/=" and "!=" as one symbol
    let (a, _d) = app();
    let s = screen(&a, 120, 40);
    let footer = s.lines().rfind(|l| l.contains("quit")).unwrap();
    for pair in ["/=", "!=", "=>", "->", "<="] {
        assert!(!footer.contains(pair), "{pair:?} in {footer:?}");
    }
}

#[test]
fn big_text_draws_block_letters_on_a_wide_terminal() {
    let (mut a, _d) = app();
    a.progress.settings.big_text = true;
    let s = screen(&a, 150, 40);
    // quadrant and full block characters make up the big letters
    let blocks = s
        .chars()
        .filter(|c| ('\u{2580}'..='\u{259f}').contains(c))
        .count();
    assert!(blocks > 50, "no block letters\n{s}");
    assert!(
        !s.contains(&first_word(&a)),
        "text drawn small as well\n{s}"
    );
    assert!(s.contains("wpm"));
}

#[test]
fn narrow_terminal_keeps_normal_text_even_with_big_text_on() {
    let (mut a, _d) = app();
    a.progress.settings.big_text = true;
    let s = screen(&a, 90, 40);
    assert!(s.contains(&first_word(&a)), "{s}");
}

#[test]
fn wide_terminal_uses_normal_text_by_default() {
    let (a, _d) = app();
    let s = screen(&a, 150, 40);
    assert!(s.contains(&first_word(&a)), "{s}");
}

#[test]
fn kitty_zoom_draws_the_text_as_runs_not_in_the_buffer() {
    let (mut a, _d) = app();
    a.zoom_ok = true;
    a.progress.settings.zoom = 2;
    let (s, runs) = screen_and_runs(&a, 150, 40);
    assert!(
        !s.contains(&first_word(&a)),
        "text also drawn normally\n{s}"
    );
    assert!(s.contains("wpm"));
    assert!(runs.iter().all(|r| r.scale == 2));
    let text: String = runs.iter().map(|r| r.text.as_str()).collect();
    assert!(text.trim_start().starts_with(&first_word(&a)), "{text:?}");
    // every row is padded to the same width so old letters get overwritten
    let mut widths = std::collections::BTreeMap::new();
    for r in &runs {
        *widths.entry(r.y).or_insert(0) += r.text.chars().count();
    }
    let w: Vec<usize> = widths.values().copied().collect();
    assert!(w.len() == 3 && w.iter().all(|x| *x == w[0]), "{w:?}");
}

#[test]
fn no_zoom_runs_without_kitty_support() {
    let (mut a, _d) = app();
    a.progress.settings.zoom = 2;
    let (s, runs) = screen_and_runs(&a, 150, 40);
    assert!(runs.is_empty());
    assert!(s.contains(&first_word(&a)));
}

#[test]
fn settings_zoom_row_says_when_kitty_is_needed() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Settings);
    assert!(screen(&a, 100, 48).contains("needs kitty"));
    a.zoom_ok = true;
    assert!(!screen(&a, 100, 48).contains("needs kitty"));
}

#[test]
fn progress_views_keep_each_mode_separate() {
    use crate::history::View;
    use crate::store::Round;
    let (mut a, _d) = app();
    for (mode, wpm) in [("base", 30), ("base", 42), ("code", 18), ("code", 22)] {
        a.progress.add_round(
            Round {
                mode: mode.into(),
                wpm,
                acc: 96,
                at_ms: NOW,
            },
            60_000,
        );
    }
    go(&mut a, Mode::Progress);
    a.progress_view = View::Base;
    let s = screen(&a, 120, 40);
    assert!(
        s.contains("baseline") && s.contains("30") && s.contains("42"),
        "{s}"
    );
    assert!(s.contains("+6"), "first 30, now (30+42)/2 = 36\n{s}");
    a.progress_view = View::Code;
    let s = screen(&a, 120, 40);
    assert!(
        s.contains("22") && !s.contains("42"),
        "code best is its own\n{s}"
    );
}

#[test]
fn results_screen_best_is_for_the_same_mode() {
    use crate::store::Round;
    let (mut a, _d) = app();
    a.progress.add_round(
        Round {
            mode: "base".into(),
            wpm: 99,
            acc: 98,
            at_ms: NOW - 100_000,
        },
        60_000,
    );
    go(&mut a, Mode::Code);
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
        "baseline best shown on a code result\n{s}"
    );
}
