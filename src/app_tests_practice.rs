// behavior tests for the practice features: progress views, mistakes,
// history, own text, test options, the ghost and letter pairs
use super::tests::{app, go, key, type_all};
use super::*;

#[test]
fn left_and_right_switch_the_progress_view() {
    use crate::history::View;
    let (mut a, _d) = app();
    go(&mut a, Mode::Progress);
    assert_eq!(a.progress_view, View::All);
    a.handle(key(cards::Key::Right), 0);
    assert_eq!(a.progress_view, View::Base);
    a.handle(key(cards::Key::Left), 0);
    a.handle(key(cards::Key::Left), 0);
    assert_eq!(
        a.progress_view,
        *View::ALL.last().unwrap(),
        "wraps around to the last view"
    );
}

#[test]
fn typing_records_key_stats_under_the_current_mode() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Code);
    let first = a.session.next_char().unwrap();
    a.handle(Input::Text(first), 0);
    assert!(a.progress.mode_stats.contains_key("code"));
    assert!(!a.progress.mode_stats.contains_key("base"));
}

#[test]
fn finishing_a_round_records_missed_words_and_counts_down_clean_ones() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Keys);
    a.progress.note_mistake("cat");
    a.progress.note_mistake("cat");
    a.session = crate::session::Session::new("the cat", None);
    a.handle(Input::Text('t'), 0);
    a.handle(Input::Text('x'), 10);
    for (i, c) in "he cat".chars().enumerate() {
        a.handle(Input::Text(c), 100 + i as u64 * 100);
    }
    assert!(a.session.is_done());
    assert_eq!(a.progress.mistakes.get("the"), Some(&1));
    assert_eq!(
        a.progress.mistakes.get("cat"),
        Some(&1),
        "typed cleanly once"
    );
}

#[test]
fn mistakes_tab_drills_the_missed_words() {
    let (mut a, _d) = app();
    for _ in 0..3 {
        a.progress.note_mistake("their");
    }
    go(&mut a, Mode::Mistakes);
    let text: String = a.session.text.iter().collect();
    assert!(text.split(' ').all(|w| w == "their"), "{text}");
    assert!(!text.is_empty());
}

#[test]
fn return_toggles_round_history_and_arrows_scroll_it() {
    let (mut a, _d) = app();
    for wpm in [30, 31, 32, 33, 34] {
        a.progress.add_round(
            store::Round {
                mode: "base".into(),
                wpm,
                acc: 95,
                at_ms: 0,
                test: None,
            },
            60_000,
        );
    }
    go(&mut a, Mode::Progress);
    assert!(!a.show_history);
    a.handle(Input::Text('\n'), 0);
    assert!(a.show_history);
    a.handle(key(cards::Key::Down), 0);
    a.handle(key(cards::Key::Down), 0);
    assert_eq!(a.history_scroll, 2);
    for _ in 0..5 {
        a.handle(key(cards::Key::Up), 0);
    }
    assert_eq!(a.history_scroll, 0);
}

#[test]
fn code_tab_types_your_own_file_when_one_is_loaded() {
    let (mut a, _d) = app();
    a.own = vec!["let total = add(1, 2);".to_string()];
    a.own_name = Some("main.rs".to_string());
    go(&mut a, Mode::Code);
    let text: String = a.session.text.iter().collect();
    assert_eq!(text, "let total = add(1, 2);");
}

#[test]
fn baseline_uses_the_chosen_test_and_files_the_round_under_it() {
    let (mut a, _d) = app();
    a.progress.settings.test = crate::baseline::Length::Words(25);
    a.progress.settings.punctuation = true;
    a.handle(Input::Tab, 0);
    a.handle(Input::BackTab, 0);
    assert_eq!(a.mode, Mode::Base);
    let text: String = a.session.text.iter().collect();
    assert_eq!(text.split(' ').count(), 25);
    type_all(&mut a, 1000);
    assert!(
        a.session.is_done(),
        "a word test ends when the words are done"
    );
    assert_eq!(a.last_round().unwrap().test.as_deref(), Some("25w +punct"));
}

#[test]
fn ghost_races_at_your_best_pace_for_the_same_test() {
    let (mut a, _d) = app();
    a.progress.add_round(
        store::Round {
            mode: "base".into(),
            wpm: 40,
            acc: 95,
            at_ms: 0,
            test: None,
        },
        60_000,
    );
    a.handle(Input::Tab, 0);
    a.handle(Input::BackTab, 0);
    assert_eq!(a.ghost_wpm, Some(40));
    let first = a.session.next_char().unwrap();
    a.handle(Input::Text(first), 1_000);
    a.tick(7_000);
    // 6 seconds at 40 wpm is 20 characters
    assert_eq!(a.ghost_at, Some(20));
}

#[test]
fn no_ghost_without_a_best_or_when_switched_off() {
    let (mut a, _d) = app();
    assert_eq!(a.ghost_wpm, None, "no rounds yet");
    a.progress.add_round(
        store::Round {
            mode: "base".into(),
            wpm: 40,
            acc: 95,
            at_ms: 0,
            test: None,
        },
        60_000,
    );
    a.progress.settings.ghost = false;
    a.handle(Input::Tab, 0);
    a.handle(Input::BackTab, 0);
    assert_eq!(a.ghost_wpm, None);
}

#[test]
fn text_rounds_save_letter_pairs_and_code_rounds_do_not() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Keys);
    a.session = crate::session::Session::new("the cat", None);
    type_all(&mut a, 0);
    assert!(a.progress.pairs.contains_key("th"));
    let before = a.progress.pairs.clone();
    go(&mut a, Mode::Code);
    type_all(&mut a, 50_000);
    assert_eq!(a.progress.pairs, before, "code has its own rhythm");
}

#[test]
fn weak_keys_drill_focuses_on_weak_pairs_too() {
    let (mut a, _d) = app();
    a.progress.pairs.insert(
        "ng".into(),
        crate::stats::KeyStat {
            n: 20,
            e: 8,
            ms: 12_000,
        },
    );
    go(&mut a, Mode::Keys);
    assert_eq!(a.focus_pairs, vec!["ng".to_string()]);
}

#[test]
fn lessons_tab_types_only_unlocked_keys() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Lessons);
    let keys = crate::lessons::unlocked(a.progress.lesson_keys);
    assert!(a.session.text.iter().all(|c| *c == ' ' || keys.contains(c)));
}

#[test]
fn a_passing_lesson_unlocks_the_next_key() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Lessons);
    let end = type_all(&mut a, 1_000);
    assert_eq!(a.progress.lesson_keys, crate::lessons::START + 1);
    assert_eq!(a.toast(end), Some("new key unlocked: G"));
}

#[test]
fn a_slow_lesson_unlocks_nothing_and_the_last_level_stays() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Lessons);
    let text: Vec<char> = a.session.text.clone();
    let mut t = 0;
    for ch in text {
        if a.session.next_char() == Some(ch) {
            a.handle(Input::Text(ch), t);
            t += 2_000;
        }
    }
    assert!(a.session.is_done());
    assert_eq!(
        a.progress.lesson_keys,
        crate::lessons::START,
        "too slow to unlock"
    );
    a.progress.lesson_keys = crate::lessons::ORDER.len();
    a.handle(Input::Text('\n'), t);
    type_all(&mut a, t + 1_000);
    assert_eq!(a.progress.lesson_keys, crate::lessons::ORDER.len());
}

#[test]
fn daily_tab_types_the_days_words_and_files_rounds_under_the_date() {
    let (mut a, _d) = app();
    let now = 1_794_009_600_000 + 3_600_000;
    a.tick(now);
    go(&mut a, Mode::Daily);
    let text: String = a.session.text.iter().collect();
    assert_eq!(text, crate::daily::text(crate::daily::day(now)));
    type_all(&mut a, now);
    assert_eq!(a.last_round().unwrap().mode, "daily");
    assert_eq!(a.last_round().unwrap().test.as_deref(), Some("2026-11-07"));
}
