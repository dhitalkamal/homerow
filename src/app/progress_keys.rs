// what keys do on the progress tab: switch views, open and scroll history
use super::{App, Input};
use crate::cards::{Key, Mods};
use crate::history::{self, View};

impl App {
    pub(super) fn progress_input(&mut self, input: Input) {
        let plain =
            |k: Key| matches!(input, Input::Combo(c) if c.mods == Mods::default() && c.key == k);
        if matches!(input, Input::Text('\n' | ' ')) {
            self.show_history = !self.show_history;
            self.history_scroll = 0;
            return;
        }
        if self.show_history && (plain(Key::Down) || matches!(input, Input::Text('j'))) {
            let rows = history::history_rows(&self.progress, self.progress_view).len();
            self.history_scroll = (self.history_scroll + 1).min(rows.saturating_sub(1));
            return;
        }
        if self.show_history && (plain(Key::Up) || matches!(input, Input::Text('k'))) {
            self.history_scroll = self.history_scroll.saturating_sub(1);
            return;
        }
        // left/right (or h/l) step through the views, wrapping round
        let step = if plain(Key::Right) || matches!(input, Input::Text('l')) {
            1
        } else if plain(Key::Left) || matches!(input, Input::Text('h')) {
            -1
        } else {
            return;
        };
        let views = View::ALL;
        let i = views
            .iter()
            .position(|v| *v == self.progress_view)
            .unwrap_or(0) as i32;
        let n = views.len() as i32;
        self.progress_view = views[((i + step + n) % n) as usize];
        self.history_scroll = 0;
    }
}
