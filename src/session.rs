// one typing round: the target text, cursor, misses and clock.
// you can't move past a wrong key, so every round trains clean presses.
use crate::stats::{self, KeyStats};

// a gap longer than this is a pause, not slow typing, so it is capped
const MAX_GAP_MS: u64 = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Correct,
    Wrong,
    Finished,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Done,
    Current,
    Missed,
    Pending,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub text: Vec<char>,
    pos: usize,
    errs: usize,
    missed_here: bool,
    start: Option<u64>,
    last: u64,
    limit_ms: Option<u64>,
    end: Option<u64>,
}

impl Session {
    pub fn new(text: &str, limit_ms: Option<u64>) -> Self {
        Session {
            text: text.chars().collect(),
            pos: 0,
            errs: 0,
            missed_here: false,
            start: None,
            last: 0,
            limit_ms,
            end: None,
        }
    }

    pub fn press(&mut self, c: char, now: u64, stats: &mut KeyStats) -> Press {
        if self.is_done() || self.tick(now) {
            return Press::Ignored;
        }
        let Some(want) = self.next_char() else {
            return Press::Ignored;
        };
        if self.start.is_none() {
            self.start = Some(now);
            self.last = now;
        }
        // letters, digits and symbols are tracked (capitals fold into their
        // lowercase key); spaces and newlines are not
        let key = want.to_ascii_lowercase();
        let tracked = key.is_ascii_graphic();
        if c != want {
            self.errs += 1;
            self.missed_here = true;
            if tracked {
                stats.entry(key).or_default().e += 1;
            }
            return Press::Wrong;
        }
        if tracked {
            let s = stats.entry(key).or_default();
            s.n += 1;
            s.ms += (now - self.last).min(MAX_GAP_MS);
        }
        self.pos += 1;
        self.last = now;
        self.missed_here = false;
        if want == '\n' {
            while self.next_char() == Some(' ') {
                self.pos += 1;
            }
        }
        if self.pos >= self.text.len() {
            self.end = Some(now);
            return Press::Finished;
        }
        Press::Correct
    }

    // ends a timed round once its limit passes. true when this call ended it.
    pub fn tick(&mut self, now: u64) -> bool {
        match (self.start, self.limit_ms, self.end) {
            (Some(start), Some(limit), None) if now >= start + limit => {
                self.end = Some(start + limit);
                true
            }
            _ => false,
        }
    }

    pub fn cell(&self, i: usize) -> Cell {
        use std::cmp::Ordering::*;
        match i.cmp(&self.pos) {
            Less => Cell::Done,
            Equal if self.missed_here => Cell::Missed,
            Equal => Cell::Current,
            Greater => Cell::Pending,
        }
    }

    pub fn is_done(&self) -> bool {
        self.end.is_some()
    }
    pub fn pos(&self) -> usize {
        self.pos
    }
    #[cfg(test)]
    pub fn errors(&self) -> usize {
        self.errs
    }
    pub fn next_char(&self) -> Option<char> {
        self.text.get(self.pos).copied()
    }

    pub fn elapsed(&self, now: u64) -> u64 {
        let Some(start) = self.start else { return 0 };
        let stop = self.end.unwrap_or(now);
        let ms = stop.saturating_sub(start);
        self.limit_ms.map_or(ms, |l| ms.min(l))
    }

    pub fn wpm(&self, now: u64) -> u32 {
        stats::wpm(self.pos, self.elapsed(now))
    }
    pub fn accuracy(&self) -> u32 {
        stats::accuracy(self.pos, self.errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::KeyStat;

    fn typed(s: &mut Session, text: &str, start: u64, gap: u64, st: &mut KeyStats) -> u64 {
        let mut t = start;
        for c in text.chars() {
            s.press(c, t, st);
            t += gap;
        }
        t
    }

    #[test]
    fn correct_key_advances_and_records_timing() {
        let mut st = KeyStats::new();
        let mut s = Session::new("ab", None);
        assert_eq!(s.press('a', 1000, &mut st), Press::Correct);
        assert_eq!(s.press('b', 1300, &mut st), Press::Finished);
        assert_eq!(st[&'a'], KeyStat { n: 1, e: 0, ms: 0 });
        assert_eq!(
            st[&'b'],
            KeyStat {
                n: 1,
                e: 0,
                ms: 300
            }
        );
    }

    #[test]
    fn wrong_key_holds_the_cursor_and_blames_the_expected_key() {
        let mut st = KeyStats::new();
        let mut s = Session::new("ab", None);
        s.press('a', 0, &mut st);
        assert_eq!(s.press('x', 100, &mut st), Press::Wrong);
        assert_eq!(s.pos(), 1);
        assert_eq!(s.cell(1), Cell::Missed);
        assert_eq!(st[&'b'].e, 1);
        assert!(!st.contains_key(&'x'));
        s.press('b', 200, &mut st);
        assert_eq!(s.errors(), 1);
        assert_eq!(s.accuracy(), 67);
    }

    #[test]
    fn capitals_count_toward_the_lowercase_key() {
        let mut st = KeyStats::new();
        let mut s = Session::new("Hi", None);
        typed(&mut s, "Hi", 0, 100, &mut st);
        assert_eq!(st[&'h'].n, 1);
    }

    #[test]
    fn long_pauses_are_capped() {
        let mut st = KeyStats::new();
        let mut s = Session::new("ab", None);
        s.press('a', 0, &mut st);
        s.press('b', 60_000, &mut st);
        assert_eq!(st[&'b'].ms, MAX_GAP_MS);
    }

    #[test]
    fn newline_skips_the_indent_that_follows() {
        let mut st = KeyStats::new();
        let mut s = Session::new("a\n    b", None);
        s.press('a', 0, &mut st);
        s.press('\n', 100, &mut st);
        assert_eq!(s.next_char(), Some('b'));
        assert_eq!(s.cell(3), Cell::Done);
    }

    #[test]
    fn cells_show_progress() {
        let mut st = KeyStats::new();
        let mut s = Session::new("abc", None);
        s.press('a', 0, &mut st);
        assert_eq!(
            [s.cell(0), s.cell(1), s.cell(2)],
            [Cell::Done, Cell::Current, Cell::Pending]
        );
    }

    #[test]
    fn timed_round_ends_at_its_limit() {
        let mut st = KeyStats::new();
        let mut s = Session::new("abcdefghij", Some(60_000));
        assert!(!s.tick(5000), "clock starts on first key, not before");
        s.press('a', 5000, &mut st);
        assert!(!s.tick(64_999));
        assert!(s.tick(65_000));
        assert!(s.is_done());
        assert_eq!(s.elapsed(99_000), 60_000);
        assert_eq!(s.press('b', 66_000, &mut st), Press::Ignored);
    }

    #[test]
    fn wpm_uses_correct_chars_over_elapsed_time() {
        let mut st = KeyStats::new();
        let text = "aaaaaaaaaa";
        let mut s = Session::new(text, None);
        typed(&mut s, text, 0, 1200, &mut st);
        // 10 chars over 10.8s
        assert_eq!(s.wpm(10_800), 11);
        assert_eq!(s.elapsed(50_000), 10_800, "elapsed freezes once finished");
    }

    #[test]
    fn symbols_and_digits_are_tracked_too() {
        let mut st = KeyStats::new();
        let mut s = Session::new("a{1 b", None);
        typed(&mut s, "a{1 b", 0, 100, &mut st);
        assert_eq!(st[&'{'].n, 1);
        assert_eq!(st[&'1'].n, 1);
        assert!(!st.contains_key(&' '), "spaces are not keys worth drilling");
    }
}
