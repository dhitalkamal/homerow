// shortcut cards: parse combos like "cmd+shift+.", match real key presses,
// and keep leitner boxes (right answer moves a card up, wrong drops it to 0).
use crate::rng::Rng;
use std::collections::BTreeMap;

// a card at this box or above counts as learned
pub const KNOWN_BOX: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Deck {
    Text,
    Mac,
    Finder,
    Zsh,
    Idea,
    VsCode,
    Browser,
    Tmux,
    Vim,
}

impl Deck {
    pub const ALL: [Deck; 9] = [
        Deck::Text,
        Deck::Mac,
        Deck::Finder,
        Deck::Zsh,
        Deck::Idea,
        Deck::VsCode,
        Deck::Browser,
        Deck::Tmux,
        Deck::Vim,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Deck::Text => "Text editing",
            Deck::Mac => "macOS",
            Deck::Finder => "Finder",
            Deck::Zsh => "Terminal (zsh)",
            Deck::Idea => "IntelliJ",
            Deck::VsCode => "VS Code",
            Deck::Browser => "Chrome / Arc",
            Deck::Tmux => "tmux",
            Deck::Vim => "vim",
        }
    }
}

// recall cards are ones macOS or the terminal takes before the app sees them
#[derive(Debug, Clone, Copy)]
pub struct Card {
    pub deck: Deck,
    pub action: &'static str,
    pub combo: &'static str,
    pub recall: bool,
}

impl Card {
    pub fn id(&self) -> String {
        format!("{:?}:{}", self.deck, self.combo).to_lowercase()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Left,
    Right,
    Up,
    Down,
    Backspace,
    Enter,
    Esc,
    Tab,
    Space,
    F(u8),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub cmd: bool,
    pub opt: bool,
    pub ctrl: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Combo {
    pub mods: Mods,
    pub key: Key,
}

// None for combos a terminal can never report, like "fn+Back" or "shift shift"
pub fn parse(s: &str) -> Option<Combo> {
    let mut mods = Mods::default();
    let mut key = None;
    for t in s.split('+') {
        match t {
            "cmd" => mods.cmd = true,
            "opt" => mods.opt = true,
            "ctrl" => mods.ctrl = true,
            "shift" => mods.shift = true,
            _ => key = Some(parse_key(t)?),
        }
    }
    // fold shifted symbols the way terminals send them, like '?' as shift+/
    Some(normalize(mods, key?))
}

fn parse_key(t: &str) -> Option<Key> {
    Some(match t {
        "Left" => Key::Left,
        "Right" => Key::Right,
        "Up" => Key::Up,
        "Down" => Key::Down,
        "Back" => Key::Backspace,
        "Enter" => Key::Enter,
        "Esc" => Key::Esc,
        "Tab" => Key::Tab,
        "Space" => Key::Space,
        _ if t.len() > 1 && t.starts_with('F') => Key::F(t[1..].parse().ok()?),
        _ if t.chars().count() == 1 => Key::Char(t.chars().next()?.to_ascii_lowercase()),
        _ => return None,
    })
}

// us layout: shifted char -> the key it lives on
const SHIFTED: [(char, char); 21] = [
    ('~', '`'),
    ('!', '1'),
    ('@', '2'),
    ('#', '3'),
    ('$', '4'),
    ('%', '5'),
    ('^', '6'),
    ('&', '7'),
    ('*', '8'),
    ('(', '9'),
    (')', '0'),
    ('_', '-'),
    ('+', '='),
    ('{', '['),
    ('}', ']'),
    ('|', '\\'),
    (':', ';'),
    ('"', '\''),
    ('<', ','),
    ('>', '.'),
    ('?', '/'),
];

// terminals often send the shifted character ('>' for shift+.), so fold it back
pub fn normalize(mut mods: Mods, key: Key) -> Combo {
    let key = match key {
        Key::Char(c) if c.is_ascii_uppercase() => {
            mods.shift = true;
            Key::Char(c.to_ascii_lowercase())
        }
        Key::Char(c) => match SHIFTED.iter().find(|(s, _)| *s == c) {
            Some(&(_, base)) => {
                mods.shift = true;
                Key::Char(base)
            }
            None => Key::Char(c),
        },
        k => k,
    };
    Combo { mods, key }
}

pub fn grade(boxes: &mut BTreeMap<String, u32>, card: &Card, ok: bool) {
    let b = boxes.entry(card.id()).or_insert(0);
    *b = if ok { *b + 1 } else { 0 };
}

// picks among the lowest-box cards in the deck filter, avoiding an instant repeat
pub fn pick(
    cards: &[Card],
    boxes: &BTreeMap<String, u32>,
    deck: Option<Deck>,
    last: Option<usize>,
    rng: &mut Rng,
) -> usize {
    let box_of = |c: &Card| boxes.get(&c.id()).copied().unwrap_or(0);
    let pool: Vec<usize> = (0..cards.len())
        .filter(|&i| deck.is_none_or(|d| cards[i].deck == d))
        .collect();
    let low = pool.iter().map(|&i| box_of(&cards[i])).min().unwrap_or(0);
    let fresh: Vec<usize> = pool
        .iter()
        .copied()
        .filter(|&i| box_of(&cards[i]) <= low && Some(i) != last)
        .collect();
    if !fresh.is_empty() {
        return *rng.pick(&fresh);
    }
    // only the last card sits at the lowest box: widen to anything but it
    let others: Vec<usize> = pool.iter().copied().filter(|&i| Some(i) != last).collect();
    *rng.pick(if others.is_empty() { &pool } else { &others })
}

pub fn known(cards: &[Card], boxes: &BTreeMap<String, u32>, deck: Deck) -> (usize, usize) {
    let in_deck: Vec<&Card> = cards.iter().filter(|c| c.deck == deck).collect();
    let k = in_deck
        .iter()
        .filter(|c| boxes.get(&c.id()).copied().unwrap_or(0) >= KNOWN_BOX)
        .count();
    (k, in_deck.len())
}

// a card's keys as steps: "ctrl+b c" is ctrl+b then c, "g g" is g twice.
// a bare single character means that character as typed, so "%" is shift+5.
// None when any step is something a terminal never reports (like "shift").
pub fn parse_seq(s: &str) -> Option<Vec<Combo>> {
    s.split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Some(normalize(Mods::default(), Key::Char(c))),
                _ => parse(part),
            }
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum Step {
    Next(usize),
    Done,
    Wrong,
}

// one key press against step `pos` of a sequence
pub fn step(seq: &[Combo], pos: usize, pressed: Combo) -> Step {
    match seq.get(pos) {
        Some(want) if *want == pressed && pos + 1 == seq.len() => Step::Done,
        Some(want) if *want == pressed => Step::Next(pos + 1),
        _ => Step::Wrong,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(cmd: bool, opt: bool, ctrl: bool, shift: bool) -> Mods {
        Mods {
            cmd,
            opt,
            ctrl,
            shift,
        }
    }
    fn card(deck: Deck, combo: &'static str) -> Card {
        Card {
            deck,
            action: "x",
            combo,
            recall: false,
        }
    }

    #[test]
    fn parses_modifiers_and_named_keys() {
        assert_eq!(
            parse("cmd+Left"),
            Some(Combo {
                mods: m(true, false, false, false),
                key: Key::Left
            })
        );
        assert_eq!(
            parse("opt+shift+Right").unwrap().mods,
            m(false, true, false, true)
        );
        assert_eq!(parse("cmd+opt+Esc").unwrap().key, Key::Esc);
        assert_eq!(parse("shift+F6").unwrap().key, Key::F(6));
        assert_eq!(parse("ctrl+A").unwrap().key, Key::Char('a'));
        assert_eq!(parse("cmd+/").unwrap().key, Key::Char('/'));
        assert_eq!(parse("opt+Back").unwrap().key, Key::Backspace);
    }

    #[test]
    fn unreportable_combos_parse_to_none() {
        assert_eq!(parse("fn+Back"), None);
        assert_eq!(parse("shift shift"), None);
    }

    #[test]
    fn shifted_symbols_fold_back_to_their_key() {
        let want = parse("cmd+shift+.").unwrap();
        assert_eq!(normalize(m(true, false, false, true), Key::Char('>')), want);
        assert_eq!(
            normalize(m(true, false, false, false), Key::Char('>')),
            want,
            "shift implied by the char"
        );
        assert_eq!(
            normalize(m(true, false, false, false), Key::Char('Z')),
            parse("cmd+shift+Z").unwrap()
        );
        assert_eq!(
            normalize(m(false, false, true, false), Key::Char('a')),
            parse("ctrl+A").unwrap()
        );
    }

    #[test]
    fn grading_moves_boxes() {
        let c = card(Deck::Zsh, "ctrl+R");
        let mut b = BTreeMap::new();
        grade(&mut b, &c, true);
        grade(&mut b, &c, true);
        assert_eq!(b[&c.id()], 2);
        grade(&mut b, &c, false);
        assert_eq!(b[&c.id()], 0);
    }

    #[test]
    fn pick_prefers_the_least_known_card() {
        let cards = [
            card(Deck::Zsh, "ctrl+A"),
            card(Deck::Zsh, "ctrl+E"),
            card(Deck::Zsh, "ctrl+R"),
        ];
        let mut b = BTreeMap::new();
        b.insert(cards[0].id(), 5);
        b.insert(cards[2].id(), 5);
        let mut rng = Rng::new(7);
        for _ in 0..20 {
            assert_eq!(pick(&cards, &b, None, None, &mut rng), 1);
        }
    }

    #[test]
    fn pick_respects_the_deck_filter_and_avoids_repeats() {
        let cards = [
            card(Deck::Mac, "cmd+,"),
            card(Deck::Zsh, "ctrl+A"),
            card(Deck::Zsh, "ctrl+E"),
        ];
        let b = BTreeMap::new();
        let mut rng = Rng::new(3);
        let mut last = None;
        for _ in 0..20 {
            let i = pick(&cards, &b, Some(Deck::Zsh), last, &mut rng);
            assert_ne!(i, 0);
            assert_ne!(Some(i), last);
            last = Some(i);
        }
    }

    #[test]
    fn known_counts_cards_at_the_known_box() {
        let cards = [
            card(Deck::Zsh, "ctrl+A"),
            card(Deck::Zsh, "ctrl+E"),
            card(Deck::Mac, "cmd+,"),
        ];
        let mut b = BTreeMap::new();
        b.insert(cards[0].id(), KNOWN_BOX);
        b.insert(cards[1].id(), 1);
        assert_eq!(known(&cards, &b, Deck::Zsh), (1, 2));
    }

    #[test]
    fn sequences_split_into_steps() {
        let ctrl_b = parse("ctrl+b").unwrap();
        let c = normalize(Mods::default(), Key::Char('c'));
        assert_eq!(parse_seq("ctrl+b c"), Some(vec![ctrl_b, c]));
        assert_eq!(parse_seq("g g").unwrap().len(), 2);
        assert_eq!(parse_seq("cmd+P"), Some(vec![parse("cmd+P").unwrap()]));
        assert_eq!(
            parse_seq("shift shift"),
            None,
            "shift alone never reaches a terminal"
        );
    }

    #[test]
    fn bare_capitals_and_symbols_mean_shift() {
        let shift5 = normalize(Mods::default(), Key::Char('%'));
        assert_eq!(parse_seq("%"), Some(vec![shift5]));
        assert_eq!(
            parse_seq("$"),
            Some(vec![normalize(Mods::default(), Key::Char('$'))])
        );
        assert_eq!(
            parse_seq("shift+G"),
            Some(vec![normalize(Mods::default(), Key::Char('G'))])
        );
    }

    #[test]
    fn steps_advance_finish_or_fail() {
        let seq = parse_seq("ctrl+b c").unwrap();
        let ctrl_b = parse("ctrl+b").unwrap();
        let c = normalize(Mods::default(), Key::Char('c'));
        assert_eq!(step(&seq, 0, ctrl_b), Step::Next(1));
        assert_eq!(step(&seq, 1, c), Step::Done);
        assert_eq!(step(&seq, 1, ctrl_b), Step::Wrong);
    }
}
