// app state and what each key press does to it. no drawing here, see ui/.
use crate::cards::{self, Combo, Deck, Key, Mods};
use crate::content::{self, CARDS};
use crate::rng::Rng;
use crate::session::Session;
use crate::stats::weak_letters;
use crate::store::{self, Progress, Round};
use std::path::PathBuf;

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
    Cards,
    Progress,
}

impl Mode {
    pub const ALL: [Mode; 5] = [
        Mode::Base,
        Mode::Keys,
        Mode::Code,
        Mode::Cards,
        Mode::Progress,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Mode::Base => "baseline",
            Mode::Keys => "weak keys",
            Mode::Code => "code",
            Mode::Cards => "shortcuts",
            Mode::Progress => "progress",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Mode::Base => "base",
            Mode::Keys => "keys",
            Mode::Code => "code",
            Mode::Cards => "cards",
            Mode::Progress => "progress",
        }
    }
    // modes where you type text and a session runs
    pub fn is_typing(self) -> bool {
        matches!(self, Mode::Base | Mode::Keys | Mode::Code)
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
    pub progress: Progress,
    pub path: PathBuf,
    pub card: usize,
    pub card_state: CardState,
    pub deck: Option<Deck>,
    pub fingers: bool,
    pub heat: bool,
    pub blind: bool,
    pub notice: Option<String>,
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
            progress,
            path,
            card: 0,
            card_state: CardState::Ask,
            deck: None,
            fingers: true,
            heat: false,
            blind: false,
            notice,
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
            Input::Combo(c) if size_step(c).is_some() => {
                let bigger = size_step(c) == Some(true);
                self.progress.density = self.progress.density.step(bigger);
                self.toast_msg = Some((
                    format!("size: {}", self.progress.density.label()),
                    now + TOAST_MS,
                ));
                self.save();
            }
            Input::Tab => self.switch(1),
            Input::BackTab => self.switch(Mode::ALL.len() - 1),
            _ if self.mode == Mode::Progress => {}
            _ if self.mode == Mode::Cards => self.card_input(input, now),
            Input::Combo(c) => self.toggle(c),
            Input::Text(ch) if self.session.is_done() => {
                if ch == '\n' {
                    self.restart();
                }
            }
            Input::Text(ch) => {
                self.session.press(ch, now, &mut self.progress.stats);
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
            }
            _ => {}
        }
    }

    fn switch(&mut self, step: usize) {
        let i = Mode::ALL.iter().position(|m| *m == self.mode).unwrap_or(0);
        self.mode = Mode::ALL[(i + step) % Mode::ALL.len()];
        self.restart();
    }

    // ctrl+F finger colors, ctrl+K weak key heatmap, ctrl+L hide key labels
    fn toggle(&mut self, c: Combo) {
        let ctrl_only = Mods {
            ctrl: true,
            ..Mods::default()
        };
        if c.mods != ctrl_only {
            return;
        }
        match c.key {
            Key::Char('f') => self.fingers = !self.fingers,
            Key::Char('k') => self.heat = !self.heat,
            Key::Char('l') => self.blind = !self.blind,
            _ => {}
        }
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
        };
        self.progress.add_round(round, self.session.elapsed(now));
        self.save();
    }

    fn card_input(&mut self, input: Input, now: u64) {
        let card = CARDS[self.card];
        if let Input::Text(b @ ('[' | ']')) = input
            && self.card_state != CardState::Judge
        {
            return self.cycle_deck(b == ']');
        }
        match (&self.card_state, input) {
            (CardState::Judge, Input::Text(ch @ ('y' | 'n'))) => {
                cards::grade(&mut self.progress.boxes, &card, ch == 'y');
                self.save();
                self.next_card();
            }
            (CardState::Judge | CardState::Done(_), _) => {}
            (CardState::Ask, Input::Text(' ')) if card.recall => self.card_state = CardState::Judge,
            (CardState::Ask, Input::Text(' ')) => {
                cards::grade(&mut self.progress.boxes, &card, false);
                self.save();
                self.card_state = CardState::Retry(None);
            }
            _ if card.recall => {}
            (state, _) => {
                let first_try = *state == CardState::Ask;
                let pressed = match input {
                    Input::Combo(c) => c,
                    Input::Text('\n') => cards::normalize(Mods::default(), Key::Enter),
                    Input::Text(' ') => cards::normalize(Mods::default(), Key::Space),
                    Input::Text(ch) => cards::normalize(Mods::default(), Key::Char(ch)),
                    _ => return,
                };
                let hit = cards::parse(card.combo) == Some(pressed);
                if first_try {
                    cards::grade(&mut self.progress.boxes, &card, hit);
                    self.save();
                }
                self.card_state = if hit {
                    CardState::Done(now + DONE_PAUSE_MS)
                } else {
                    CardState::Retry(Some(pressed))
                };
            }
        }
    }

    fn cycle_deck(&mut self, forward: bool) {
        let opts: Vec<Option<Deck>> = std::iter::once(None)
            .chain(Deck::ALL.iter().copied().map(Some))
            .collect();
        let i = opts.iter().position(|d| *d == self.deck).unwrap_or(0);
        let n = opts.len();
        self.deck = opts[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }];
        self.next_card();
    }

    fn restart(&mut self) {
        self.recorded = false;
        match self.mode {
            Mode::Base => {
                self.session = Session::new(
                    &content::baseline_text(&mut self.rng, 150),
                    Some(BASELINE_MS),
                )
            }
            Mode::Keys => {
                self.focus = weak_letters(&self.progress.stats, 3);
                self.session =
                    Session::new(&content::weak_text(&mut self.rng, &self.focus, 24), None);
            }
            Mode::Code => self.session = Session::new(content::code_text(&mut self.rng), None),
            Mode::Cards => self.next_card(),
            Mode::Progress => {}
        }
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

    pub fn last_round(&self) -> Option<&Round> {
        self.progress.rounds.last()
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

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
