// starting and finishing rounds: the text each mode types, what a finished
// round saves (stats, pairs, mistakes, lesson unlocks) and what it races
use super::{App, LESSON_ITEMS, Mode, TOAST_MS};
use crate::baseline;
use crate::content;
use crate::history;
use crate::lessons;
use crate::session::Session;
use crate::stats::weak_letters;
use crate::store::Round;

impl App {
    pub(super) fn record_if_done(&mut self, now: u64) {
        if !self.session.is_done() || self.recorded {
            return;
        }
        self.recorded = true;
        let round = Round {
            mode: self.mode.tag().into(),
            wpm: self.session.wpm(now),
            acc: self.session.accuracy(),
            at_ms: now,
            test: self.test_label(),
        };
        let (wpm, acc) = (round.wpm, round.acc);
        self.progress.add_round(round, self.session.elapsed(now));
        // guided lessons: a good enough round opens the next key
        if self.mode == Mode::Lessons
            && lessons::passes(wpm, acc)
            && self.progress.lesson_keys < lessons::ORDER.len()
        {
            self.progress.lesson_keys += 1;
            let key = lessons::newest(self.progress.lesson_keys).to_ascii_uppercase();
            self.toast_msg = Some((format!("new key unlocked: {key}"), now + 2 * TOAST_MS));
        }
        // letter pairs only from text: code has its own rhythm
        if matches!(self.mode, Mode::Base | Mode::Keys) {
            for (p, s) in self.session.pairs() {
                let e = self.progress.pairs.entry(p.clone()).or_default();
                e.n += s.n;
                e.e += s.e;
                e.ms += s.ms;
            }
        }
        for w in self.session.missed_words() {
            self.progress.note_mistake(&w);
        }
        for w in self.session.clean_words() {
            self.progress.note_clean(&w);
        }
        self.save();
    }

    pub(super) fn restart(&mut self) {
        self.recorded = false;
        self.ghost_at = None;
        match self.mode {
            Mode::Base => {
                let s = self.progress.settings;
                let text = baseline::text(&mut self.rng, s.test, s.punctuation, s.numbers);
                self.session = Session::new(&text, s.test.limit_ms());
            }
            Mode::Keys => {
                self.focus_pairs = crate::stats::weak_pairs(&self.progress.pairs, 2);
                self.focus = weak_letters(
                    &history::merged(&self.progress, &[history::BASE, history::KEYS]),
                    3,
                );
                self.session = Session::new(
                    &content::weak_text(&mut self.rng, &self.focus, &self.focus_pairs, 24),
                    None,
                );
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
            Mode::Lessons => {
                let text = lessons::text(&mut self.rng, self.progress.lesson_keys, LESSON_ITEMS);
                self.session = Session::new(&text, None);
            }
            Mode::Daily => {
                let text = crate::daily::text(crate::daily::day(self.clock));
                self.session = Session::new(&text, None);
            }
            Mode::Cards => self.next_card(),
            Mode::Progress | Mode::Settings => {}
        }
        self.ghost_wpm = self.ghost_pace();
    }

    // what this mode's rounds are filed under: the baseline test, or the
    // date for the daily challenge; other modes have none
    pub fn test_label(&self) -> Option<String> {
        let s = self.progress.settings;
        match self.mode {
            Mode::Base => Some(baseline::label(s.test, s.punctuation, s.numbers)),
            Mode::Daily => Some(crate::store::day_key(self.clock)),
            _ => None,
        }
    }

    // the best to race this round, if racing is on and there is one
    pub(super) fn ghost_pace(&self) -> Option<u32> {
        if !self.progress.settings.ghost || !self.mode.is_typing() {
            return None;
        }
        let test = self.test_label().unwrap_or_default();
        history::best_for(&self.progress, self.mode.tag(), &test)
    }
}
