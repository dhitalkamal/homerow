// your own shortcut decks from a plain text file:
//
//   # comments start with a hash
//   [Figma]
//   Duplicate selection = cmd+D
//   Group selection = cmd+G
//
// the text ends up on screen, so control characters are dropped, and size
// caps keep a huge file from freezing the app. bad lines are reported with
// their line number and skipped; they never block the rest.

use crate::cards::{Card, Deck, parse_seq};

// one card from the file
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub deck: String,
    pub action: String,
    pub keys: String,
}

pub const MAX_CARDS: usize = 500;
pub const MAX_LINE: usize = 200;
pub const MAX_NAME: usize = 40;

// the cards it could read, plus one message per skipped line
pub fn parse(text: &str) -> (Vec<Entry>, Vec<String>) {
    let mut cards: Vec<Entry> = Vec::new();
    let mut errs = Vec::new();
    let mut deck: Option<String> = None;
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        if raw.chars().count() > MAX_LINE {
            errs.push(format!("line {n}: too long (over {MAX_LINE} characters)"));
            continue;
        }
        let clean: String = raw.chars().filter(|c| !c.is_control()).collect();
        let line = clean.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let name: String = name.trim().chars().take(MAX_NAME).collect();
            if name.is_empty() {
                errs.push(format!("line {n}: empty deck name"));
            } else {
                deck = Some(name);
            }
            continue;
        }
        let Some(current) = &deck else {
            errs.push(format!("line {n}: put a [deck name] line before the cards"));
            continue;
        };
        let parts = line.split_once('=').map(|(a, k)| (a.trim(), k.trim()));
        let Some((action, keys)) = parts.filter(|(a, k)| !a.is_empty() && !k.is_empty()) else {
            errs.push(format!("line {n}: expected 'action = keys'"));
            continue;
        };
        if cards.iter().any(|c| &c.deck == current && c.keys == keys) {
            errs.push(format!("line {n}: {keys} is already in [{current}]"));
            continue;
        }
        if cards.len() == MAX_CARDS {
            errs.push(format!(
                "line {n}: only the first {MAX_CARDS} cards are used"
            ));
            break;
        }
        cards.push(Entry {
            deck: current.clone(),
            action: action.to_string(),
            keys: keys.to_string(),
        });
    }
    (cards, errs)
}

// turn entries into cards. the file is read once at startup and the cards
// live for the whole run, so their text is leaked into 'static strings; that
// lets custom cards share the built-in Card type without copying it around
pub fn to_cards(entries: Vec<Entry>) -> Vec<Card> {
    let mut names: Vec<&'static str> = Vec::new();
    entries
        .into_iter()
        .map(|e| {
            let deck = match names.iter().find(|n| **n == e.deck) {
                Some(n) => *n,
                None => {
                    let n: &'static str = Box::leak(e.deck.into_boxed_str());
                    names.push(n);
                    n
                }
            };
            let recall = parse_seq(&e.keys).is_none();
            Card {
                deck: Deck::Custom(deck),
                action: Box::leak(e.action.into_boxed_str()),
                combo: Box::leak(e.keys.into_boxed_str()),
                recall,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(deck: &str, action: &str, keys: &str) -> Entry {
        Entry {
            deck: deck.into(),
            action: action.into(),
            keys: keys.into(),
        }
    }

    #[test]
    fn sections_and_cards() {
        let (cards, errs) =
            parse("# mine\n[Figma]\nDuplicate = cmd+D\n\n[tmux]\nNew window = ctrl+a c\n");
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(
            cards,
            vec![
                entry("Figma", "Duplicate", "cmd+D"),
                entry("tmux", "New window", "ctrl+a c")
            ]
        );
    }

    #[test]
    fn bad_lines_are_reported_with_line_numbers_and_skipped() {
        let (cards, errs) = parse(
            "Orphan = cmd+X\n[Figma]\nno equals sign\nGroup = cmd+G\n = cmd+Z\nEmpty keys =\n",
        );
        assert_eq!(cards, vec![entry("Figma", "Group", "cmd+G")]);
        assert_eq!(errs.len(), 4, "{errs:?}");
        assert!(
            errs[0].starts_with("line 1:") && errs[0].contains("[deck name]"),
            "{errs:?}"
        );
        assert!(
            errs[1].starts_with("line 3:") && errs[1].contains("action = keys"),
            "{errs:?}"
        );
        assert!(
            errs[2].starts_with("line 5:") && errs[3].starts_with("line 6:"),
            "{errs:?}"
        );
    }

    #[test]
    fn duplicate_keys_in_a_deck_are_skipped() {
        let (cards, errs) = parse("[A]\nOne = cmd+D\nTwo = cmd+D\n[B]\nThree = cmd+D\n");
        assert_eq!(cards.len(), 2, "same keys in another deck are fine");
        assert!(
            errs[0].starts_with("line 3:") && errs[0].contains("cmd+D"),
            "{errs:?}"
        );
    }

    #[test]
    fn control_characters_are_dropped_and_sizes_capped() {
        let (cards, _) = parse("[Fig\u{1b}[2Jma]\nDup\u{7}licate = cmd+D\n");
        assert_eq!(cards, vec![entry("Fig[2Jma", "Duplicate", "cmd+D")]);
        let long = format!("[A]\nx = {}\n", "a".repeat(MAX_LINE + 1));
        let (cards, errs) = parse(&long);
        assert!(cards.is_empty() && errs[0].contains("too long"), "{errs:?}");
        let many: String = std::iter::once("[A]\n".to_string())
            .chain((0..MAX_CARDS + 5).map(|i| format!("card {i} = cmd+{i}\n")))
            .collect();
        assert_eq!(parse(&many).0.len(), MAX_CARDS);
    }

    #[test]
    fn entries_become_cards_with_their_own_ids() {
        let cards = to_cards(vec![
            entry("Figma", "Duplicate", "cmd+D"),
            entry("Figma", "Odd", "fn+Back"),
        ]);
        assert_eq!(cards[0].id(), "my:figma:cmd+d");
        assert_eq!(cards[0].deck.label(), "Figma");
        assert!(!cards[0].recall);
        assert!(
            cards[1].recall,
            "keys a terminal cannot report become recall cards"
        );
    }
}
