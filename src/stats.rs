// typing math: speed, accuracy and per-key weakness. no terminal code here.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// per letter: correct hits, misses, total ms spent on correct hits
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct KeyStat {
    pub n: u32,
    pub e: u32,
    pub ms: u64,
}

pub type KeyStats = BTreeMap<char, KeyStat>;
// same numbers for two letters typed one after the other, like "th"
pub type PairStats = BTreeMap<String, KeyStat>;

// a key needs this many correct hits before its numbers are trusted
pub const MIN_SAMPLES: u32 = 5;

pub fn wpm(chars: usize, ms: u64) -> u32 {
    if ms == 0 {
        return 0;
    }
    ((chars as f64 / 5.0) / (ms as f64 / 60_000.0)).round() as u32
}

pub fn accuracy(ok: usize, bad: usize) -> u32 {
    if ok + bad == 0 {
        return 100;
    }
    (100.0 * ok as f64 / (ok + bad) as f64).round() as u32
}

pub fn avg_ms(stats: &KeyStats) -> f64 {
    let (n, ms) = stats
        .values()
        .fold((0u64, 0u64), |(n, ms), s| (n + s.n as u64, ms + s.ms));
    if n == 0 { 0.0 } else { ms as f64 / n as f64 }
}

// higher = weaker. misses weigh heavily, slowness is relative to your own average.
// untried keys get 1.2 so they come up before keys you already type cleanly.
pub fn weakness(stat: Option<&KeyStat>, avg_ms: f64) -> f64 {
    match stat {
        Some(s) if s.n >= MIN_SAMPLES => {
            let miss_rate = s.e as f64 / (s.n + s.e) as f64;
            let rel_speed = (s.ms as f64 / s.n as f64) / avg_ms.max(1.0);
            miss_rate * 6.0 + rel_speed
        }
        _ => 1.2,
    }
}

pub fn weak_letters(stats: &KeyStats, k: usize) -> Vec<char> {
    let avg = avg_ms(stats);
    let mut scored: Vec<(char, f64)> = ('a'..='z')
        .map(|c| (c, weakness(stats.get(&c), avg)))
        .collect();
    // stable sort keeps alphabetical order among ties
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.into_iter().take(k).map(|(c, _)| c).collect()
}

// the k slowest or most missed letter pairs that have enough samples
pub fn weak_pairs(pairs: &PairStats, k: usize) -> Vec<String> {
    let (n, ms) = pairs
        .values()
        .fold((0u64, 0u64), |(n, ms), s| (n + s.n as u64, ms + s.ms));
    let avg = if n == 0 { 0.0 } else { ms as f64 / n as f64 };
    let mut scored: Vec<(&String, f64)> = pairs
        .iter()
        .filter(|(_, s)| s.n >= MIN_SAMPLES)
        .map(|(p, s)| (p, weakness(Some(s), avg)))
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.into_iter().take(k).map(|(p, _)| p.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(n: u32, e: u32, ms: u64) -> KeyStat {
        KeyStat { n, e, ms }
    }

    #[test]
    fn wpm_counts_five_chars_as_a_word() {
        assert_eq!(wpm(250, 60_000), 50);
        assert_eq!(wpm(125, 30_000), 50);
    }

    #[test]
    fn wpm_is_zero_before_any_time_passes() {
        assert_eq!(wpm(10, 0), 0);
    }

    #[test]
    fn accuracy_is_percent_of_correct_presses() {
        assert_eq!(accuracy(95, 5), 95);
        assert_eq!(accuracy(0, 0), 100);
        assert_eq!(accuracy(2, 1), 67);
    }

    #[test]
    fn avg_ms_is_weighted_by_hits() {
        let mut s = KeyStats::new();
        s.insert('a', st(10, 0, 1000));
        s.insert('b', st(30, 0, 9000));
        assert_eq!(avg_ms(&s), 250.0);
        assert_eq!(avg_ms(&KeyStats::new()), 0.0);
    }

    #[test]
    fn untried_key_gets_a_middling_score() {
        assert_eq!(weakness(None, 200.0), 1.2);
        assert_eq!(weakness(Some(&st(4, 3, 900)), 200.0), 1.2);
    }

    #[test]
    fn sloppy_slow_key_scores_above_clean_fast_key() {
        let clean = weakness(Some(&st(50, 0, 5000)), 200.0);
        let sloppy = weakness(Some(&st(50, 10, 20000)), 200.0);
        assert!((clean - 0.5).abs() < 1e-9);
        assert!((sloppy - 3.0).abs() < 1e-9);
    }

    #[test]
    fn weak_letters_picks_the_worst_known_keys_first() {
        let mut s = KeyStats::new();
        for c in 'a'..='z' {
            s.insert(c, st(50, 0, 5000));
        }
        s.insert('q', st(50, 20, 30000));
        s.insert('z', st(50, 10, 20000));
        assert_eq!(weak_letters(&s, 2), vec!['q', 'z']);
    }

    #[test]
    fn weak_letters_surfaces_untried_keys_over_clean_ones() {
        let mut s = KeyStats::new();
        for c in 'a'..='y' {
            s.insert(c, st(50, 0, 5000));
        }
        assert_eq!(weak_letters(&s, 1), vec!['z']);
    }

    #[test]
    fn weak_pairs_are_the_slowest_and_most_missed_with_enough_samples() {
        let mut p = PairStats::new();
        p.insert("th".into(), st(20, 0, 2000));
        p.insert("ng".into(), st(20, 6, 9000));
        p.insert("er".into(), st(20, 0, 6000));
        p.insert("qz".into(), st(2, 2, 3000));
        assert_eq!(weak_pairs(&p, 2), vec!["ng".to_string(), "er".to_string()]);
    }
}
