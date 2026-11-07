// sample progress for `homerow --demo`: two weeks of made-up rounds so the
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
            };
            p.add_round(round, 4 * 60_000);
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
        p.stats.insert(
            c,
            KeyStat {
                n: 60,
                e,
                ms: 60 * avg,
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
        let weak = weak_letters(&p.stats, 3);
        assert!(weak.contains(&'q') && weak.contains(&'z'), "{weak:?}");
        assert!(streak(&p.days, NOW) >= 5);
    }

    #[test]
    fn demo_decks_are_partly_learned() {
        let p = progress(NOW);
        let (k, n) = known(CARDS, &p.boxes, Deck::Zsh);
        assert!(k > 0 && k < n, "{k}/{n}");
    }
}
