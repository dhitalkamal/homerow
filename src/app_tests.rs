// behavior tests for App: modes, rounds, shortcut cards, toggles
use super::*;

fn app() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let a = App::new(dir.path().join("p.json"), 42);
    (a, dir)
}
fn combo(s: &str) -> Input {
    Input::Combo(cards::parse(s).unwrap())
}
fn type_all(a: &mut App, start: u64) -> u64 {
    let text: String = a.session.text.iter().collect();
    let mut t = start;
    for ch in text.chars().collect::<Vec<_>>() {
        if a.session.is_done() {
            break;
        }
        // indentation after a newline is skipped by the session
        if a.session.next_char() == Some(ch) {
            a.handle(Input::Text(ch), t);
            t += 100;
        }
    }
    t
}
fn go(a: &mut App, mode: Mode) {
    while a.mode != mode {
        a.handle(Input::Tab, 0);
    }
}
fn key(k: cards::Key) -> Input {
    Input::Combo(cards::Combo {
        mods: cards::Mods::default(),
        key: k,
    })
}
fn set_card(a: &mut App, combo: &str) {
    a.card = CARDS.iter().position(|c| c.combo == combo).unwrap();
    a.card_state = CardState::Ask;
}

#[test]
fn tab_cycles_modes_and_backtab_goes_back() {
    let (mut a, _d) = app();
    a.handle(Input::Tab, 0);
    assert_eq!(a.mode, Mode::Keys);
    a.handle(Input::BackTab, 0);
    a.handle(Input::BackTab, 0);
    assert_eq!(a.mode, Mode::Settings);
    a.handle(Input::BackTab, 0);
    assert_eq!(a.mode, Mode::Progress);
}

#[test]
fn progress_tab_ignores_typing_and_card_keys() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Progress);
    let boxes = a.progress.boxes.clone();
    for i in [
        Input::Text('a'),
        Input::Text(' '),
        Input::Text('\n'),
        combo("ctrl+R"),
    ] {
        a.handle(i, 0);
    }
    a.tick(999_999);
    assert!(a.progress.rounds.is_empty());
    assert_eq!(a.progress.boxes, boxes);
    assert!(a.progress.stats.is_empty());
}

#[test]
fn finished_round_is_recorded_and_saved() {
    let (mut a, dir) = app();
    a.handle(Input::Tab, 0);
    a.handle(Input::Tab, 0);
    assert_eq!(a.mode, Mode::Code);
    type_all(&mut a, 1000);
    assert!(a.session.is_done());
    let r = a.last_round().unwrap();
    assert_eq!((r.mode.as_str(), r.acc), ("code", 100));
    let (saved, _) = store::load(&dir.path().join("p.json"));
    assert_eq!(saved.rounds.len(), 1);
}

#[test]
fn enter_after_a_round_starts_the_next_one() {
    let (mut a, _d) = app();
    a.handle(Input::Tab, 0);
    a.handle(Input::Tab, 0);
    type_all(&mut a, 0);
    a.handle(Input::Text('\n'), 99_000);
    assert!(!a.session.is_done());
    assert_eq!(a.session.pos(), 0);
}

#[test]
fn baseline_ends_on_tick_after_sixty_seconds() {
    let (mut a, _d) = app();
    let first = a.session.next_char().unwrap();
    a.handle(Input::Text(first), 0);
    a.tick(BASELINE_MS);
    assert!(a.session.is_done());
    assert_eq!(a.last_round().unwrap().mode, "base");
}

#[test]
fn right_combo_moves_the_card_up() {
    let (mut a, _d) = app();
    a.mode = Mode::Cards;
    set_card(&mut a, "ctrl+R");
    a.handle(combo("ctrl+R"), 0);
    assert_eq!(a.card_state, CardState::Done(DONE_PAUSE_MS));
    assert_eq!(a.progress.boxes[&CARDS[a.card].id()], 1);
    a.tick(DONE_PAUSE_MS);
    assert_eq!(a.card_state, CardState::Ask);
}

#[test]
fn wrong_combo_resets_the_card_and_waits_for_the_right_one() {
    let (mut a, _d) = app();
    a.mode = Mode::Cards;
    set_card(&mut a, "ctrl+R");
    let id = CARDS[a.card].id();
    a.progress.boxes.insert(id.clone(), 2);
    a.handle(combo("ctrl+E"), 0);
    assert_eq!(a.progress.boxes[&id], 0);
    assert!(matches!(a.card_state, CardState::Retry(Some(_))));
    a.handle(combo("ctrl+R"), 10);
    assert_eq!(a.progress.boxes[&id], 0, "a retry does not earn credit");
    assert!(matches!(a.card_state, CardState::Done(_)));
}

#[test]
fn recall_card_reveals_on_space_then_takes_y_or_n() {
    let (mut a, _d) = app();
    a.mode = Mode::Cards;
    set_card(&mut a, "cmd+Tab");
    let id = CARDS[a.card].id();
    a.handle(Input::Text(' '), 0);
    assert_eq!(a.card_state, CardState::Judge);
    a.handle(Input::Text('y'), 0);
    assert_eq!(a.progress.boxes[&id], 1);
    assert_eq!(a.card_state, CardState::Ask);
}

#[test]
fn space_on_a_live_card_shows_the_answer_and_counts_as_a_miss() {
    let (mut a, _d) = app();
    a.mode = Mode::Cards;
    set_card(&mut a, "ctrl+W");
    a.handle(Input::Text(' '), 0);
    assert_eq!(a.card_state, CardState::Retry(None));
    assert_eq!(a.progress.boxes[&CARDS[a.card].id()], 0);
}

#[test]
fn brackets_cycle_the_deck_filter() {
    let (mut a, _d) = app();
    a.mode = Mode::Cards;
    a.handle(Input::Text(']'), 0);
    assert_eq!(a.deck, Some(Deck::Text));
    a.handle(Input::Text('['), 0);
    assert_eq!(a.deck, None);
    a.handle(Input::Text('['), 0);
    assert_eq!(a.deck, Some(Deck::Idea));
    assert_eq!(CARDS[a.card].deck, Deck::Idea);
}

#[test]
fn esc_quits() {
    let (mut a, _d) = app();
    a.handle(Input::Esc, 0);
    assert!(a.quit);
}

#[test]
fn ctrl_equals_and_minus_change_density_and_save_it() {
    let (mut a, dir) = app();
    a.handle(combo("ctrl+="), 1000);
    assert_eq!(a.progress.density, store::Density::Spacious);
    let (saved, _) = store::load(&dir.path().join("p.json"));
    assert_eq!(saved.density, store::Density::Spacious);
    a.handle(combo("ctrl+-"), 1000);
    a.handle(combo("ctrl+-"), 1000);
    a.handle(combo("ctrl+-"), 1000);
    assert_eq!(a.progress.density, store::Density::Compact);
    assert_eq!(a.session.pos(), 0, "size keys never count as typing");
}

#[test]
fn density_change_shows_a_short_toast() {
    let (mut a, _d) = app();
    a.handle(combo("ctrl+="), 1000);
    assert_eq!(a.toast(1000), Some("size: spacious"));
    assert_eq!(a.toast(1000 + TOAST_MS - 1), Some("size: spacious"));
    assert_eq!(a.toast(1000 + TOAST_MS), None);
}

#[test]
fn density_keys_work_on_every_tab_without_grading_cards() {
    let (mut a, _d) = app();
    a.mode = Mode::Cards;
    set_card(&mut a, "ctrl+R");
    a.handle(combo("ctrl+-"), 0);
    assert_eq!(a.card_state, CardState::Ask);
    assert!(a.progress.boxes.is_empty());
    assert_eq!(a.progress.density, store::Density::Compact);
    a.mode = Mode::Progress;
    a.handle(combo("ctrl+="), 0);
    assert_eq!(a.progress.density, store::Density::Comfortable);
}

#[test]
fn settings_cursor_moves_and_stops_at_the_ends() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Settings);
    a.handle(key(cards::Key::Down), 0);
    a.handle(Input::Text('j'), 0);
    assert_eq!(a.settings_row, 2);
    for _ in 0..5 {
        a.handle(key(cards::Key::Up), 0);
    }
    assert_eq!(a.settings_row, 0);
    for _ in 0..20 {
        a.handle(Input::Text('j'), 0);
    }
    assert_eq!(a.settings_row, crate::settings::ITEMS.len() - 1);
}

#[test]
fn space_toggles_the_keyboard_and_saves() {
    let (mut a, dir) = app();
    go(&mut a, Mode::Settings);
    a.handle(Input::Text(' '), 0);
    assert!(a.progress.settings.keyboard);
    let (saved, _) = store::load(&dir.path().join("p.json"));
    assert!(saved.settings.keyboard);
    a.handle(Input::Text('\n'), 0);
    assert!(!a.progress.settings.keyboard);
}

#[test]
fn left_and_right_change_the_selected_setting() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Settings);
    let size_row = crate::settings::ITEMS
        .iter()
        .position(|i| *i == crate::settings::Item::Size)
        .unwrap();
    for _ in 0..size_row {
        a.handle(key(cards::Key::Down), 0);
    }
    a.handle(key(cards::Key::Right), 0);
    assert_eq!(a.progress.density, store::Density::Spacious);
    a.handle(key(cards::Key::Left), 0);
    assert_eq!(a.progress.density, store::Density::Comfortable);
}

#[test]
fn typing_on_the_settings_tab_does_nothing() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Settings);
    a.handle(Input::Text('a'), 0);
    a.tick(999_999);
    assert!(a.progress.stats.is_empty() && a.progress.rounds.is_empty());
}

#[test]
fn size_keys_step_from_what_auto_is_showing() {
    let (mut a, _d) = app();
    a.screen = (150, 50);
    assert_eq!(a.progress.density, store::Density::Auto);
    // auto shows large at this size, so one step down is spacious
    a.handle(combo("ctrl+-"), 0);
    assert_eq!(a.progress.density, store::Density::Spacious);
}

#[test]
fn zoom_keys_step_between_1x_and_3x_when_kitty_supports_it() {
    let (mut a, dir) = app();
    a.zoom_ok = true;
    for _ in 0..4 {
        a.handle(combo("ctrl+]"), 0);
    }
    assert_eq!(a.progress.settings.zoom, 3);
    assert_eq!(a.toast(0), Some("zoom: 3x"));
    a.handle(combo("ctrl+["), 0);
    assert_eq!(a.progress.settings.zoom, 2);
    let (saved, _) = store::load(&dir.path().join("p.json"));
    assert_eq!(saved.settings.zoom, 2);
}

#[test]
fn zoom_keys_explain_when_the_terminal_cannot_zoom() {
    let (mut a, _d) = app();
    a.handle(combo("ctrl+]"), 0);
    assert_eq!(a.progress.settings.zoom, 1);
    assert_eq!(a.toast(0), Some("zoom needs kitty 0.40 or newer"));
}

#[test]
fn zoom_keys_are_never_graded_as_shortcut_cards() {
    let (mut a, _d) = app();
    a.zoom_ok = true;
    a.mode = Mode::Cards;
    set_card(&mut a, "ctrl+R");
    a.handle(combo("ctrl+]"), 0);
    assert_eq!(a.card_state, CardState::Ask);
    assert!(a.progress.boxes.is_empty());
}

#[test]
fn settings_zoom_row_does_nothing_without_kitty_support() {
    let (mut a, _d) = app();
    go(&mut a, Mode::Settings);
    let row = crate::settings::ITEMS
        .iter()
        .position(|i| *i == crate::settings::Item::Zoom)
        .unwrap();
    for _ in 0..row {
        a.handle(key(cards::Key::Down), 0);
    }
    a.handle(Input::Text(' '), 0);
    a.handle(key(cards::Key::Right), 0);
    assert_eq!(a.progress.settings.zoom, 1);
    assert_eq!(a.toast(0), Some("zoom needs kitty 0.40 or newer"));
    a.zoom_ok = true;
    a.handle(Input::Text(' '), 0);
    assert_eq!(a.progress.settings.zoom, 2);
}
