// the daily challenge: the same 50 words for everyone on a given utc day,
// picked by a generator seeded with the day number (no server needed)
use crate::content::words;
use crate::rng::Rng;

pub const WORDS_PER_DAY: usize = 50;
const DAY_MS: u64 = 86_400_000;

// days since 1970-01-01 (utc) for a unix time in ms
pub fn day(now_ms: u64) -> u64 {
    now_ms / DAY_MS
}

// the day's text; same day, same words
pub fn text(day: u64) -> String {
    // spread neighbouring days apart so day n and n+1 share nothing
    let mut rng = Rng::new(day.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5DEE_CE66);
    let list = words();
    (0..WORDS_PER_DAY)
        .map(|_| *rng.pick(&list))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOV7: u64 = 1_794_009_600_000;

    #[test]
    fn day_counts_whole_utc_days() {
        assert_eq!(
            day(NOV7),
            day(NOV7 + DAY_MS - 1),
            "same day until utc midnight"
        );
        assert_eq!(day(NOV7 + DAY_MS), day(NOV7) + 1);
    }

    #[test]
    fn same_day_same_words_next_day_different() {
        let d = day(NOV7);
        assert_eq!(text(d), text(d));
        assert_ne!(text(d), text(d + 1));
    }

    #[test]
    fn daily_text_is_fifty_plain_words_from_the_list() {
        let t = text(day(NOV7));
        let list = words();
        assert_eq!(t.split(' ').count(), WORDS_PER_DAY);
        assert!(t.split(' ').all(|w| list.contains(&w)), "{t}");
    }
}
