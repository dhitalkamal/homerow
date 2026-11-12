// sample progress for `keyup --demo`: two weeks of made-up rounds so the
// dashboard can be previewed. never written to the real progress file.
use crate::cards::KNOWN_BOX;
use crate::content::CARDS;
use crate::rng::Rng;
use crate::stats::KeyStat;
use crate::store::{Progress, Round};

const DAY: u64 = 86_400_000;
const HOUR: u64 = 3_600_000;

pub fn progress(now: u64) -> Progress {
    let mut rng = Rng::new(7);
    let mut p = Progress::default();
    let modes = ["base", "keys", "code"];
    // 14 days back, with a 2 day gap so the streak is 9 days, not 14
    for d in (0..14u64).rev().filter(|d| *d != 9 && *d != 10) {
        for i in 0..3u64 {
            let noise = rng.below(7) as f64 - 3.0;
            let wpm = (28.0 + (13 - d) as f64 * 1.4 + noise).round() as u32;
            let round = Round {
                mode: modes[(i as usize + d as usize) % 3].into(),
                wpm,
                acc: 93 + rng.below(7) as u32,
                at_ms: now - d * DAY - (3 - i) * HOUR,
                test: None,
            };
            // 3 rounds of 6 minutes: each demo day meets the default 15 minute goal
            p.add_round(round, 6 * 60_000);
        }
    }
    // clean keys, with a few clearly slow and sloppy ones
    for c in 'a'..='z' {
        let (e, avg) = match c {
            'q' => (14, 420),
            'z' => (10, 380),
            'x' => (6, 330),
            'b' => (4, 300),
            _ => (1, 210 + rng.below(40) as u64),
        };
        p.mode_stats.entry("base".into()).or_default().insert(
            c,
            KeyStat {
                n: 60,
                e,
                ms: 60 * avg,
            },
        );
    }
    // code: brackets and symbols are the slow part
    for (c, e, avg) in [
        ('{', 6, 520),
        ('}', 4, 480),
        ('(', 3, 400),
        (')', 3, 390),
        (';', 5, 450),
        ('_', 7, 560),
        ('=', 2, 330),
    ] {
        p.mode_stats.entry("code".into()).or_default().insert(
            c,
            KeyStat {
                n: 30,
                e,
                ms: 30 * avg,
            },
        );
    }
    // letter pairs: most are quick, a few common ones drag
    for (pair, e, avg) in [
        ("th", 1, 180),
        ("he", 0, 170),
        ("in", 1, 190),
        ("er", 2, 260),
        ("ng", 6, 420),
        ("ou", 3, 300),
    ] {
        p.pairs.insert(
            pair.into(),
            KeyStat {
                n: 40,
                e,
                ms: 40 * avg,
            },
        );
    }
    // some cards learned per deck, the rest early
    for (i, card) in CARDS.iter().enumerate() {
        let b = if i % 3 == 0 {
            KNOWN_BOX + 1
        } else {
            rng.below(3) as u32
        };
        p.boxes.insert(card.id(), b);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Deck, known};
    use crate::stats::weak_letters;
    use crate::store::streak;

    const NOW: u64 = 1_794_009_600_000 + 15 * 3_600_000;

    #[test]
    fn demo_has_two_weeks_of_rising_rounds() {
        let p = progress(NOW);
        assert!(p.rounds.len() >= 20);
        assert!(p.rounds.windows(2).all(|w| w[0].at_ms < w[1].at_ms));
        assert!(p.rounds.iter().all(|r| r.at_ms <= NOW));
        let typing: Vec<u32> = p
            .rounds
            .iter()
            .filter(|r| r.mode != "cards")
            .map(|r| r.wpm)
            .collect();
        let (first, last) = (
            typing[..5].iter().sum::<u32>(),
            typing[typing.len() - 5..].iter().sum::<u32>(),
        );
        assert!(last > first, "speed should trend up: {first} -> {last}");
    }

    #[test]
    fn demo_has_obvious_weak_keys_and_a_live_streak() {
        let p = progress(NOW);
        let weak = weak_letters(&crate::history::merged(&p, &["base", "keys"]), 3);
        assert!(weak.contains(&'q') && weak.contains(&'z'), "{weak:?}");
        assert!(streak(&p.days, NOW) >= 5);
        let pairs = crate::stats::weak_pairs(&p.pairs, 2);
        assert_eq!(pairs.first().map(String::as_str), Some("ng"), "{pairs:?}");
    }

    #[test]
    fn demo_decks_are_partly_learned() {
        let p = progress(NOW);
        let (k, n) = known(CARDS, &p.boxes, Deck::Zsh);
        assert!(k > 0 && k < n, "{k}/{n}");
    }
}
