// app state and what each key press does to it. no drawing here, see ui/.
use crate::baseline;
use crate::cards::{self, Combo, Deck, Key};
use crate::content::{self, CARDS};
use crate::history;
use crate::rng::Rng;
use crate::session::Session;
use crate::stats::weak_letters;
use crate::store::{self, Progress, Round};
use std::path::PathBuf;

// the default baseline test, a plain minute
#[cfg(test)]
const BASELINE_MS: u64 = 60_000;
const DONE_PAUSE_MS: u64 = 650;
// how long a size change message stays in the footer
pub const TOAST_MS: u64 = 1500;

// keys after the terminal layer has folded them (see input.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Text(char),
    Combo(Combo),
    Tab,
    BackTab,
    Esc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Base,
    Keys,
    Code,
    Mistakes,
    Cards,
    Progress,
    Settings,
}

impl Mode {
    pub const ALL: [Mode; 7] = [
        Mode::Base,
        Mode::Keys,
        Mode::Code,
        Mode::Mistakes,
        Mode::Cards,
        Mode::Progress,
        Mode::Settings,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Mode::Base => "baseline",
            Mode::Keys => "weak keys",
            Mode::Code => "code",
            Mode::Mistakes => "mistakes",
            Mode::Cards => "shortcuts",
            Mode::Progress => "progress",
            Mode::Settings => "settings",
        }
    }
    pub fn tag(self) -> &'static str {
        match self {
            Mode::Base => history::BASE,
            Mode::Keys => history::KEYS,
            Mode::Code => history::CODE,
            Mode::Mistakes => history::MISTAKES,
            Mode::Cards => "cards",
            Mode::Progress => "progress",
            Mode::Settings => "settings",
        }
    }
    // modes where you type text and a session runs
    pub fn is_typing(self) -> bool {
        matches!(self, Mode::Base | Mode::Keys | Mode::Code | Mode::Mistakes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardState {
    Ask,
    Judge,
    Retry(Option<Combo>),
    Done(u64),
}

pub struct App {
    pub mode: Mode,
    pub session: Session,
    pub focus: Vec<char>,
    // race your best: the pace of your best for this mode and test, and the
    // letter the ghost is on (updated every tick while a round runs)
    pub ghost_wpm: Option<u32>,
    pub ghost_at: Option<usize>,
    pub progress: Progress,
    pub path: PathBuf,
    pub card: usize,
    pub card_state: CardState,
    pub deck: Option<Deck>,
    pub notice: Option<String>,
    // selected row on the settings tab
    pub settings_row: usize,
    // which mode the progress tab shows
    pub progress_view: history::View,
    // progress tab shows the round list instead of the overview
    pub show_history: bool,
    // rounds from your own file (--file); the code tab uses them when set
    pub own: Vec<String>,
    pub own_name: Option<String>,
    pub history_scroll: usize,
    // last known terminal size, so auto size can be stepped from
    pub screen: (u16, u16),
    // the terminal understands kitty text sizing (checked at startup)
    pub zoom_ok: bool,
    pub quit: bool,
    // the current round is already saved, so a late tick can't save it twice
    recorded: bool,
    // short footer message and when it disappears
    toast_msg: Option<(String, u64)>,
    rng: Rng,
}

impl App {
    pub fn new(path: PathBuf, seed: u64) -> Self {
        let (progress, notice) = store::load(&path);
        let mut app = App {
            mode: Mode::Base,
            session: Session::new("", None),
            focus: Vec::new(),
            ghost_wpm: None,
            ghost_at: None,
            progress,
            path,
            card: 0,
            card_state: CardState::Ask,
            deck: None,
            notice,
            settings_row: 0,
            progress_view: history::View::All,
            show_history: false,
            own: Vec::new(),
            own_name: None,
            history_scroll: 0,
            screen: (100, 40),
            zoom_ok: false,
            quit: false,
            recorded: false,
            toast_msg: None,
            rng: Rng::new(seed),
        };
        app.restart();
        app
    }

    pub fn handle(&mut self, input: Input, now: u64) {
        match input {
            Input::Esc => self.quit = true,
            Input::Combo(c) if zoom_step(c).is_some() => {
                let msg = if self.zoom_ok {
                    let z = self.progress.settings.zoom;
                    self.progress.settings.zoom = if zoom_step(c) == Some(true) {
                        (z + 1).min(crate::settings::MAX_ZOOM)
                    } else {
                        z.saturating_sub(1).max(1)
                    };
                    self.save();
                    format!("zoom: {}x", self.progress.settings.zoom)
                } else {
                    "zoom needs kitty 0.40 or newer".to_string()
                };
                self.toast_msg = Some((msg, now + TOAST_MS));
            }
            Input::Combo(c) if size_step(c).is_some() => {
                let bigger = size_step(c) == Some(true);
                let now_showing = self.progress.density.resolve(self.screen.0, self.screen.1);
                self.progress.density = now_showing.step(bigger);
                self.toast_msg = Some((
                    format!("size: {}", self.progress.density.label()),
                    now + TOAST_MS,
                ));
                self.save();
            }
            Input::Tab => self.switch(1),
            Input::BackTab => self.switch(Mode::ALL.len() - 1),
            _ if self.mode == Mode::Progress => self.progress_input(input),
            _ if self.mode == Mode::Settings => self.settings_input(input, now),
            _ if self.mode == Mode::Cards => self.card_input(input, now),
            Input::Combo(_) => {}
            Input::Text(ch) if self.session.is_done() => {
                if ch == '\n' {
                    self.restart();
                }
            }
            Input::Text(ch) => {
                let stats = self
                    .progress
                    .mode_stats
                    .entry(self.mode.tag().into())
                    .or_default();
                self.session.press(ch, now, stats);
                self.record_if_done(now);
            }
        }
    }

    pub fn tick(&mut self, now: u64) {
        match self.card_state {
            CardState::Done(until) if self.mode == Mode::Cards && now >= until => self.next_card(),
            _ if self.mode.is_typing() => {
                self.session.tick(now);
                self.record_if_done(now);
                self.ghost_at = match self.ghost_wpm {
                    Some(w) if self.session.elapsed(now) > 0 && !self.session.is_done() => {
                        let last = self.session.text.len().saturating_sub(1);
                        Some(history::ghost_pos(w, self.session.elapsed(now)).min(last))
                    }
                    _ => None,
                };
            }
            _ => {}
        }
    }

    fn switch(&mut self, step: usize) {
        let i = Mode::ALL.iter().position(|m| *m == self.mode).unwrap_or(0);
        self.mode = Mode::ALL[(i + step) % Mode::ALL.len()];
        self.restart();
    }

    fn record_if_done(&mut self, now: u64) {
        if !self.session.is_done() || self.recorded {
            return;
        }
        self.recorded = true;
        let round = Round {
            mode: self.mode.tag().into(),
            wpm: self.session.wpm(now),
            acc: self.session.accuracy(),
            at_ms: now,
            test: (self.mode == Mode::Base).then(|| {
                let s = self.progress.settings;
                baseline::label(s.test, s.punctuation, s.numbers)
            }),
        };
        self.progress.add_round(round, self.session.elapsed(now));
        for w in self.session.missed_words() {
            self.progress.note_mistake(&w);
        }
        for w in self.session.clean_words() {
            self.progress.note_clean(&w);
        }
        self.save();
    }

    fn restart(&mut self) {
        self.recorded = false;
        self.ghost_at = None;
        match self.mode {
            Mode::Base => {
                let s = self.progress.settings;
                let text = baseline::text(&mut self.rng, s.test, s.punctuation, s.numbers);
                self.session = Session::new(&text, s.test.limit_ms());
            }
            Mode::Keys => {
                self.focus = weak_letters(
                    &history::merged(&self.progress, &[history::BASE, history::KEYS]),
                    3,
                );
                self.session =
                    Session::new(&content::weak_text(&mut self.rng, &self.focus, 24), None);
            }
            Mode::Code if !self.own.is_empty() => {
                let chunk = self.rng.pick(&self.own).clone();
                self.session = Session::new(&chunk, None);
            }
            Mode::Code => self.session = Session::new(content::code_text(&mut self.rng), None),
            Mode::Mistakes => {
                let text = content::mistakes_text(&mut self.rng, &self.progress.mistakes, 24);
                self.session = Session::new(&text, None);
            }
            Mode::Cards => self.next_card(),
            Mode::Progress | Mode::Settings => {}
        }
        self.ghost_wpm = self.ghost_pace();
    }

    // the best to race this round, if racing is on and there is one
    fn ghost_pace(&self) -> Option<u32> {
        if !self.progress.settings.ghost || !self.mode.is_typing() {
            return None;
        }
        let s = self.progress.settings;
        let test = if self.mode == Mode::Base {
            baseline::label(s.test, s.punctuation, s.numbers)
        } else {
            String::new()
        };
        history::best_for(&self.progress, self.mode.tag(), &test)
    }

    fn next_card(&mut self) {
        let last = Some(self.card);
        self.card = cards::pick(CARDS, &self.progress.boxes, self.deck, last, &mut self.rng);
        self.card_state = CardState::Ask;
    }

    fn save(&mut self) {
        if let Err(e) = store::save(&self.path, &self.progress) {
            self.notice = Some(format!(
                "could not save progress to {}: {e}",
                self.path.display()
            ));
        }
    }

    pub fn toast(&self, now: u64) -> Option<&str> {
        self.toast_msg
            .as_ref()
            .filter(|(_, until)| now < *until)
            .map(|(m, _)| m.as_str())
    }

    // key stats behind the typing screen right now: code has its own,
    // baseline and weak keys share text stats
    pub fn key_stats_now(&self) -> crate::stats::KeyStats {
        if self.mode == Mode::Code {
            history::merged(&self.progress, &[history::CODE])
        } else {
            history::merged(&self.progress, &[history::BASE, history::KEYS])
        }
    }

    pub fn last_round(&self) -> Option<&Round> {
        self.progress.rounds.last()
    }
}

// ctrl+] zooms the typing text in, ctrl+[ out (kitty only)
fn zoom_step(c: Combo) -> Option<bool> {
    let ctrl_only = c.mods.ctrl && !c.mods.cmd && !c.mods.opt && !c.mods.shift;
    match c.key {
        Key::Char(']') if ctrl_only => Some(true),
        Key::Char('[') if ctrl_only => Some(false),
        _ => None,
    }
}

// ctrl+= (or ctrl and the + key) is bigger, ctrl+- is smaller
fn size_step(c: Combo) -> Option<bool> {
    let ctrl = c.mods.ctrl && !c.mods.cmd && !c.mods.opt;
    match c.key {
        Key::Char('=') if ctrl => Some(true),
        Key::Char('-') if ctrl && !c.mods.shift => Some(false),
        _ => None,
    }
}

// shortcut card keys and settings keys live in their own files
mod card_keys;
mod progress_keys;
mod settings_keys;

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
