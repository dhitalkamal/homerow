// what keys do on the settings tab: move the cursor, flip a switch, change size
use super::{App, Input, TOAST_MS};
use crate::cards::Key;
use crate::settings::{self, ITEMS, Item};

impl App {
    pub(super) fn settings_input(&mut self, input: Input, now: u64) {
        let last = ITEMS.len() - 1;
        let item = ITEMS[self.settings_row.min(last)];
        // zoom only works in terminals with kitty text sizing; say so instead
        // of storing a value that would do nothing
        let changes = matches!(input, Input::Text(' ' | '\n'))
            || matches!(input, Input::Combo(c) if matches!(c.key, Key::Left | Key::Right) && c.mods == Default::default());
        if item == Item::Zoom && changes && !self.zoom_ok {
            self.toast_msg = Some(("zoom needs kitty 0.40 or newer".into(), now + TOAST_MS));
            return;
        }
        match input {
            Input::Text('k') => self.settings_row = self.settings_row.saturating_sub(1),
            Input::Text('j') => self.settings_row = (self.settings_row + 1).min(last),
            Input::Text(' ' | '\n') => {
                settings::toggle(item, &mut self.progress);
                self.save();
            }
            Input::Combo(c) if c.mods == Default::default() => match c.key {
                Key::Up => self.settings_row = self.settings_row.saturating_sub(1),
                Key::Down => self.settings_row = (self.settings_row + 1).min(last),
                Key::Left | Key::Right => {
                    settings::adjust(item, &mut self.progress, c.key == Key::Right, self.screen);
                    self.save();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
