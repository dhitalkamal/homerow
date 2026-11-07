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
    assert_eq!(a.mode, Mode::Progress);
    a.handle(Input::BackTab, 0);
    assert_eq!(a.mode, Mode::Cards);
}

#[test]
fn progress_tab_ignores_typing_and_card_keys() {
    let (mut a, _d) = app();
    a.handle(Input::BackTab, 0);
    assert_eq!(a.mode, Mode::Progress);
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
fn ctrl_toggles_change_the_keyboard_view() {
    let (mut a, _d) = app();
    a.handle(combo("ctrl+F"), 0);
    a.handle(combo("ctrl+K"), 0);
    a.handle(combo("ctrl+L"), 0);
    assert!(!a.fingers && a.heat && a.blind);
    assert_eq!(a.session.pos(), 0, "toggles never count as typing");
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
