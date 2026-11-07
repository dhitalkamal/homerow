// questions about your progress, answered per mode: which rounds, how far
// you came since the first one, and which keys are weak in a given view.
use crate::stats::{KeyStat, KeyStats};
use crate::store::{Progress, Round};

// round tags as saved in progress.json
pub const BASE: &str = "base";
pub const KEYS: &str = "keys";
pub const CODE: &str = "code";
pub const MISTAKES: &str = "mistakes";
pub const LESSONS: &str = "lessons";

// what the progress tab is showing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    All,
    Base,
    Keys,
    Code,
    Mistakes,
    Lessons,
}

impl View {
    pub const ALL: [View; 6] = [
        View::All,
        View::Base,
        View::Keys,
        View::Code,
        View::Mistakes,
        View::Lessons,
    ];
    pub fn label(self) -> &'static str {
        match self {
            View::All => "all modes",
            View::Base => "baseline",
            View::Keys => "weak keys",
            View::Code => "code",
            View::Mistakes => "mistakes",
            View::Lessons => "lessons",
        }
    }
    // round tags this view covers
    pub fn modes(self) -> &'static [&'static str] {
        match self {
            View::All => &[BASE, KEYS, CODE, MISTAKES, LESSONS],
            View::Base => &[BASE],
            View::Keys => &[KEYS],
            View::Code => &[CODE],
            View::Mistakes => &[MISTAKES],
            View::Lessons => &[LESSONS],
        }
    }
}

// first round against now, for one view
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    pub first: u32,
    // average of the last 3 rounds, so one lucky round does not count as progress
    pub now: u32,
    pub best: u32,
    pub rounds: usize,
}

impl Change {
    pub fn delta(&self) -> i64 {
        self.now as i64 - self.first as i64
    }
}

pub fn rounds(p: &Progress, view: View) -> Vec<&Round> {
    let modes = view.modes();
    p.rounds
        .iter()
        .filter(|r| modes.contains(&r.mode.as_str()))
        .collect()
}

pub fn change(p: &Progress, view: View) -> Option<Change> {
    let rs = rounds(p, view);
    let first = rs.first()?.wpm;
    let recent = &rs[rs.len().saturating_sub(3)..];
    let now = recent.iter().map(|r| r.wpm).sum::<u32>() / recent.len() as u32;
    let best = rs.iter().map(|r| r.wpm).max().unwrap_or(0);
    Some(Change {
        first,
        now,
        best,
        rounds: rs.len(),
    })
}

// key stats summed over the given round tags
pub fn merged(p: &Progress, modes: &[&str]) -> KeyStats {
    let mut out = KeyStats::new();
    for m in modes {
        for (c, s) in p.mode_stats.get(*m).into_iter().flatten() {
            let e: &mut KeyStat = out.entry(*c).or_default();
            e.n += s.n;
            e.e += s.e;
            e.ms += s.ms;
        }
    }
    out
}

// the stats a view's weak keys come from. text (baseline plus weak keys) for
// the all view, since that is what the weak keys drill trains.
pub fn view_stats(p: &Progress, view: View) -> KeyStats {
    match view {
        View::All => merged(p, &[BASE, KEYS]),
        v => merged(p, v.modes()),
    }
}

// every round in the view, newest first
pub fn history_rows(p: &Progress, view: View) -> Vec<&Round> {
    let mut rs = rounds(p, view);
    rs.reverse();
    rs
}

// how long ago, in the largest unit that fits
pub fn ago(now: u64, then: u64) -> String {
    let secs = now.saturating_sub(then) / 1000;
    match secs {
        0..60 => "just now".to_string(),
        60..3600 => format!("{}m ago", secs / 60),
        3600..86_400 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86_400),
    }
}

// the test a round counts under; baseline rounds from before test options
// all ran the plain 60 second test
pub fn test_of(r: &Round) -> String {
    match (&r.test, r.mode.as_str()) {
        (Some(t), _) => t.clone(),
        (None, BASE) => "60s".to_string(),
        (None, _) => String::new(),
    }
}

// your best speed for one mode and test, the pace the ghost runs at
pub fn best_for(p: &Progress, mode: &str, test: &str) -> Option<u32> {
    p.rounds
        .iter()
        .filter(|r| r.mode == mode && test_of(r) == test)
        .map(|r| r.wpm)
        .max()
}

// characters a typist at `wpm` has typed after `elapsed_ms` (a word is 5)
pub fn ghost_pos(wpm: u32, elapsed_ms: u64) -> usize {
    (wpm as u64 * 5 * elapsed_ms / 60_000) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(mode: &str, wpm: u32, at: u64) -> Round {
        Round {
            mode: mode.into(),
            wpm,
            acc: 95,
            at_ms: at,
            test: None,
        }
    }

    fn sample() -> Progress {
        let mut p = Progress::default();
        for (i, w) in [20, 22, 30, 32, 34].iter().enumerate() {
            p.add_round(round(BASE, *w, i as u64 * 1000), 60_000);
        }
        p.add_round(round(CODE, 15, 9_000), 60_000);
        p.add_round(round(CODE, 25, 10_000), 60_000);
        p
    }

    #[test]
    fn rounds_are_filtered_by_view() {
        let p = sample();
        assert_eq!(rounds(&p, View::Base).len(), 5);
        assert_eq!(rounds(&p, View::Code).len(), 2);
        assert_eq!(rounds(&p, View::Keys).len(), 0);
        assert_eq!(rounds(&p, View::All).len(), 7);
    }

    #[test]
    fn change_compares_the_first_round_with_the_recent_average() {
        let c = change(&sample(), View::Base).unwrap();
        assert_eq!((c.first, c.now, c.best, c.rounds), (20, 32, 34, 5));
        assert_eq!(c.delta(), 12);
    }

    #[test]
    fn change_with_fewer_than_three_rounds_averages_what_there_is() {
        let c = change(&sample(), View::Code).unwrap();
        assert_eq!((c.first, c.now), (15, 20));
        assert_eq!(change(&sample(), View::Keys), None);
    }

    #[test]
    fn merged_sums_key_stats_across_modes() {
        let mut p = Progress::default();
        p.mode_stats.entry(BASE.into()).or_default().insert(
            'a',
            KeyStat {
                n: 5,
                e: 1,
                ms: 500,
            },
        );
        p.mode_stats.entry(KEYS.into()).or_default().insert(
            'a',
            KeyStat {
                n: 3,
                e: 0,
                ms: 300,
            },
        );
        p.mode_stats.entry(CODE.into()).or_default().insert(
            '{',
            KeyStat {
                n: 4,
                e: 2,
                ms: 900,
            },
        );
        let text = view_stats(&p, View::All);
        assert_eq!(
            text[&'a'],
            KeyStat {
                n: 8,
                e: 1,
                ms: 800
            }
        );
        assert!(
            !text.contains_key(&'{'),
            "code symbols stay out of text stats"
        );
        assert_eq!(view_stats(&p, View::Code)[&'{'].e, 2);
    }

    #[test]
    fn ago_reads_like_a_person_would_say_it() {
        let now = 10 * 86_400_000;
        assert_eq!(ago(now, now - 20_000), "just now");
        assert_eq!(ago(now, now - 5 * 60_000), "5m ago");
        assert_eq!(ago(now, now - 3 * 3_600_000), "3h ago");
        assert_eq!(ago(now, now - 2 * 86_400_000), "2d ago");
    }

    #[test]
    fn history_lists_newest_first_for_the_view() {
        let p = sample();
        let rows = history_rows(&p, View::Code);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].wpm, 25, "newest first");
    }

    #[test]
    fn best_is_per_mode_and_test() {
        let mut p = sample();
        p.add_round(
            Round {
                mode: BASE.into(),
                wpm: 90,
                acc: 99,
                at_ms: 20_000,
                test: Some("15s".into()),
            },
            15_000,
        );
        assert_eq!(
            best_for(&p, BASE, "60s"),
            Some(34),
            "old rounds count as 60s"
        );
        assert_eq!(best_for(&p, BASE, "15s"), Some(90));
        assert_eq!(best_for(&p, CODE, ""), Some(25));
        assert_eq!(best_for(&p, KEYS, ""), None);
    }

    #[test]
    fn ghost_moves_at_the_best_pace() {
        // 60 wpm is 300 characters a minute
        assert_eq!(ghost_pos(60, 60_000), 300);
        assert_eq!(ghost_pos(60, 6_000), 30);
        assert_eq!(ghost_pos(0, 60_000), 0);
    }
}
