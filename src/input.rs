// crossterm key events -> app inputs. plain typing stays text, anything with
// cmd, opt or ctrl (or a special key) becomes a combo for the shortcut cards.
use crate::app::Input;
use crate::cards::{Key, Mods, normalize};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn map(ev: KeyEvent) -> Option<Input> {
    if ev.kind == KeyEventKind::Release {
        return None;
    }
    let m = ev.modifiers;
    let mods = Mods {
        cmd: m.contains(KeyModifiers::SUPER),
        opt: m.contains(KeyModifiers::ALT),
        ctrl: m.contains(KeyModifiers::CONTROL),
        shift: m.contains(KeyModifiers::SHIFT),
    };
    let plain = !mods.cmd && !mods.opt && !mods.ctrl;
    let key = match ev.code {
        KeyCode::Char(c) if plain => return Some(Input::Text(c)),
        KeyCode::Enter if plain && !mods.shift => return Some(Input::Text('\n')),
        KeyCode::Tab if mods == Mods::default() => return Some(Input::Tab),
        KeyCode::BackTab if plain => return Some(Input::BackTab),
        KeyCode::Esc if mods == Mods::default() => return Some(Input::Esc),
        KeyCode::Char(' ') => Key::Space,
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Tab | KeyCode::BackTab => Key::Tab,
        KeyCode::F(n) => Key::F(n),
        _ => return None,
    };
    Some(Input::Combo(normalize(mods, key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::parse;

    fn ev(code: KeyCode, m: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, m)
    }
    fn combo(s: &str) -> Option<Input> {
        Some(Input::Combo(parse(s).unwrap()))
    }
    const NONE: KeyModifiers = KeyModifiers::NONE;

    #[test]
    fn plain_and_shifted_chars_are_text() {
        assert_eq!(map(ev(KeyCode::Char('a'), NONE)), Some(Input::Text('a')));
        assert_eq!(
            map(ev(KeyCode::Char('A'), KeyModifiers::SHIFT)),
            Some(Input::Text('A'))
        );
        assert_eq!(map(ev(KeyCode::Char(' '), NONE)), Some(Input::Text(' ')));
        assert_eq!(map(ev(KeyCode::Enter, NONE)), Some(Input::Text('\n')));
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(map(ev(KeyCode::Tab, NONE)), Some(Input::Tab));
        assert_eq!(
            map(ev(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(Input::BackTab)
        );
        assert_eq!(map(ev(KeyCode::Esc, NONE)), Some(Input::Esc));
    }

    #[test]
    fn modified_keys_become_combos() {
        assert_eq!(
            map(ev(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            combo("ctrl+R")
        );
        assert_eq!(
            map(ev(KeyCode::Left, KeyModifiers::SUPER)),
            combo("cmd+Left")
        );
        assert_eq!(
            map(ev(KeyCode::Char('e'), KeyModifiers::ALT)),
            combo("opt+E")
        );
        assert_eq!(
            map(ev(
                KeyCode::Char('>'),
                KeyModifiers::SUPER | KeyModifiers::SHIFT
            )),
            combo("cmd+shift+.")
        );
        assert_eq!(
            map(ev(KeyCode::F(6), KeyModifiers::SHIFT)),
            combo("shift+F6")
        );
        assert_eq!(
            map(ev(KeyCode::Backspace, KeyModifiers::ALT)),
            combo("opt+Back")
        );
        assert_eq!(
            map(ev(KeyCode::Esc, KeyModifiers::SUPER | KeyModifiers::ALT)),
            combo("cmd+opt+Esc")
        );
    }

    #[test]
    fn plain_special_keys_are_combos_without_mods() {
        let Some(Input::Combo(c)) = map(ev(KeyCode::Backspace, NONE)) else {
            panic!()
        };
        assert_eq!((c.mods, c.key), (Mods::default(), Key::Backspace));
    }

    #[test]
    fn releases_and_unknown_keys_are_dropped() {
        let mut up = ev(KeyCode::Char('a'), NONE);
        up.kind = KeyEventKind::Release;
        assert_eq!(map(up), None);
        assert_eq!(map(ev(KeyCode::Home, NONE)), None);
    }
}
