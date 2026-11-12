// baseline test options: how long a test runs (a time or a word count) and
// whether it mixes in punctuation and numbers, plus the text it types
use crate::content::words;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Length {
    // seconds
    Time(u16),
    // words to type
    Words(u16),
}

impl Default for Length {
    fn default() -> Self {
        Length::Time(60)
    }
}

// the order space and left/right step through
pub const LENGTHS: [Length; 7] = [
    Length::Time(15),
    Length::Time(30),
    Length::Time(60),
    Length::Time(120),
    Length::Words(25),
    Length::Words(50),
    Length::Words(100),
];

impl Length {
    pub fn short(self) -> String {
        match self {
            Length::Time(s) => format!("{s}s"),
            Length::Words(w) => format!("{w}w"),
        }
    }
    // a timed test ends at its limit, a word test when the words are done
    pub fn limit_ms(self) -> Option<u64> {
        match self {
            Length::Time(s) => Some(s as u64 * 1000),
            Length::Words(_) => None,
        }
    }
    // words to generate: enough that nobody runs out before the clock does
    pub fn words(self) -> usize {
        match self {
            // 3 words a second is 180 wpm, faster than almost anyone types
            Length::Time(s) => s as usize * 3,
            Length::Words(w) => w as usize,
        }
    }
    // one step through LENGTHS, wrapping round
    pub fn step(self, forward: bool) -> Length {
        let n = LENGTHS.len();
        let i = LENGTHS.iter().position(|l| *l == self).unwrap_or(2);
        LENGTHS[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }]
    }
}

// what a round is filed under, so only like is compared with like
pub fn label(len: Length, punctuation: bool, numbers: bool) -> String {
    let mut out = len.short();
    if punctuation {
        out.push_str(" +punct");
    }
    if numbers {
        out.push_str(" +num");
    }
    out
}

// common words, optionally in sentences (capital start, commas, full stops)
// and with numbers mixed in
pub fn text(rng: &mut Rng, len: Length, punctuation: bool, numbers: bool) -> String {
    let pool = words();
    let n = len.words();
    let mut out: Vec<String> = Vec::with_capacity(n);
    // words left in the current sentence, only used with punctuation
    let mut left = 0;
    for i in 0..n {
        let mut w = if numbers && rng.below(8) == 0 {
            (rng.below(9999) + 1).to_string()
        } else {
            rng.pick(&pool).to_string()
        };
        if punctuation {
            if left == 0 {
                left = 5 + rng.below(8);
                w = capitalize(&w);
            }
            left -= 1;
            if left == 0 || i == n - 1 {
                w.push(if rng.below(6) == 0 { '?' } else { '.' });
                left = 0;
            } else if rng.below(7) == 0 {
                w.push(',');
            }
        }
        out.push(w);
    }
    out.join(" ")
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_say_length_and_extras() {
        assert_eq!(label(Length::Time(60), false, false), "60s");
        assert_eq!(label(Length::Words(25), true, false), "25w +punct");
        assert_eq!(label(Length::Time(30), true, true), "30s +punct +num");
    }

    #[test]
    fn timed_tests_have_a_limit_and_word_tests_do_not() {
        assert_eq!(Length::Time(30).limit_ms(), Some(30_000));
        assert_eq!(Length::Words(50).limit_ms(), None);
        assert_eq!(Length::Words(50).words(), 50);
        assert!(Length::Time(120).words() >= 360, "180 wpm for 2 minutes");
    }

    #[test]
    fn lengths_step_and_wrap() {
        assert_eq!(Length::Time(60).step(true), Length::Time(120));
        assert_eq!(Length::Words(100).step(true), Length::Time(15));
        assert_eq!(Length::Time(15).step(false), Length::Words(100));
    }

    #[test]
    fn plain_text_is_lowercase_words_only() {
        let t = text(&mut Rng::new(3), Length::Words(25), false, false);
        assert_eq!(t.split(' ').count(), 25);
        assert!(t.chars().all(|c| c.is_ascii_lowercase() || c == ' '), "{t}");
        let w = words();
        assert!(
            t.split(' ').all(|x| w.contains(&x)),
            "only words from the list"
        );
    }

    #[test]
    fn punctuation_makes_sentences() {
        let t = text(&mut Rng::new(3), Length::Words(50), true, false);
        assert_eq!(t.split(' ').count(), 50);
        assert!(t.chars().next().unwrap().is_ascii_uppercase(), "{t}");
        assert!(t.ends_with('.') || t.ends_with('?'), "{t}");
        assert!(t.contains(". "), "{t}");
    }

    #[test]
    fn numbers_mix_in_digits() {
        let t = text(&mut Rng::new(5), Length::Words(100), false, true);
        assert_eq!(t.split(' ').count(), 100);
        assert!(
            t.split(' ').any(|w| w.chars().all(|c| c.is_ascii_digit())),
            "{t}"
        );
    }
}
