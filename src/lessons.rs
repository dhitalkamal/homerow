// guided lessons: keys unlock one at a time, starting with the home row.
// a round that clears the bar (accuracy and speed) opens the next key.
use crate::content::words;
use crate::rng::Rng;

// home row first, then roughly by how common letters are in english, so real
// words show up early
pub const ORDER: [char; 27] = [
    'f', 'j', 'd', 'k', 's', 'l', 'a', ';', 'g', 'h', 'e', 'i', 'r', 'u', 't', 'n', 'o', 'c', 'w',
    'm', 'v', 'b', 'p', 'y', 'x', 'q', 'z',
];
// everyone starts with the eight home row keys
pub const START: usize = 8;
// the bar a lesson round has to clear to unlock the next key
pub const PASS_ACC: u32 = 95;
pub const PASS_WPM: u32 = 20;

pub fn unlocked(level: usize) -> &'static [char] {
    &ORDER[..level.clamp(START, ORDER.len())]
}

// the key unlocked most recently, the one lessons lean on
pub fn newest(level: usize) -> char {
    let keys = unlocked(level);
    keys[keys.len() - 1]
}

pub fn passes(wpm: u32, acc: u32) -> bool {
    acc >= PASS_ACC && wpm >= PASS_WPM
}

// about `n` items using only unlocked keys: real words once there are enough
// of them, letter groups before that; the newest key comes up often
pub fn text(rng: &mut Rng, level: usize, n: usize) -> String {
    let keys = unlocked(level);
    let new = newest(level);
    let real: Vec<&str> = words()
        .into_iter()
        .filter(|w| w.chars().all(|c| keys.contains(&c)))
        .collect();
    if real.len() >= MIN_REAL_WORDS {
        // words with the newest key count several times over
        let mut pool = Vec::new();
        for w in real {
            let weight = if w.contains(new) { NEWEST_WEIGHT } else { 1 };
            pool.extend(std::iter::repeat_n(w, weight));
        }
        return (0..n)
            .map(|_| *rng.pick(&pool))
            .collect::<Vec<_>>()
            .join(" ");
    }
    (0..n)
        .map(|_| group(rng, keys, new))
        .collect::<Vec<_>>()
        .join(" ")
}

// a 2 to 5 key group from the unlocked keys, half of them with the newest key
fn group(rng: &mut Rng, keys: &[char], new: char) -> String {
    let len = 2 + rng.below(4);
    let mut g: Vec<char> = (0..len).map(|_| *rng.pick(keys)).collect();
    if rng.below(2) == 0 {
        let at = rng.below(len);
        g[at] = new;
    }
    g.into_iter().collect()
}

// real words take over once this many can be typed with the unlocked keys
const MIN_REAL_WORDS: usize = 10;
// how many times a word with the newest key goes into the pick pool
const NEWEST_WEIGHT: usize = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_covers_every_letter_and_semicolon_once() {
        let mut keys: Vec<char> = ORDER.to_vec();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), 27);
        assert!(('a'..='z').all(|c| ORDER.contains(&c)));
    }

    #[test]
    fn lessons_start_on_the_home_row() {
        assert_eq!(unlocked(START), &['f', 'j', 'd', 'k', 's', 'l', 'a', ';']);
        assert_eq!(newest(START), ';');
        assert_eq!(newest(9), 'g');
        assert_eq!(
            unlocked(99).len(),
            27,
            "levels past the end stay at the end"
        );
    }

    #[test]
    fn the_bar_is_95_percent_and_20_wpm() {
        assert!(passes(20, 95));
        assert!(!passes(19, 99));
        assert!(!passes(60, 94));
    }

    #[test]
    fn early_lessons_use_only_home_row_keys() {
        let t = text(&mut Rng::new(2), START, 20);
        assert_eq!(t.split(' ').count(), 20);
        assert!(
            t.chars().all(|c| c == ' ' || unlocked(START).contains(&c)),
            "{t}"
        );
    }

    #[test]
    fn later_lessons_use_real_words_from_unlocked_keys_and_favor_the_newest() {
        let level = 18; // through 'c'
        let t = text(&mut Rng::new(7), level, 40);
        let keys = unlocked(level);
        assert!(
            t.split(' ').all(|w| w.chars().all(|c| keys.contains(&c))),
            "{t}"
        );
        let all = words();
        assert!(
            t.split(' ').all(|w| all.contains(&w)),
            "real words at this level: {t}"
        );
        let with_newest = t.split(' ').filter(|w| w.contains('c')).count();
        assert!(with_newest >= 10, "newest key in {with_newest} of 40: {t}");
    }
}
