// what keys do on the settings tab: move the cursor, flip a switch, change size
use super::{App, Input};
use crate::cards::Key;
use crate::settings::{self, ITEMS};

impl App {
    pub(super) fn settings_input(&mut self, input: Input) {
        let last = ITEMS.len() - 1;
        let item = ITEMS[self.settings_row.min(last)];
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
                    settings::adjust(item, &mut self.progress, c.key == Key::Right);
                    self.save();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
