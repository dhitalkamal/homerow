// practice material and the builders that turn it into drill text
use crate::cards::{Card, Deck};
use crate::rng::Rng;

pub const WORDS: &str = "the of and to in is you that it he was for on are as with his they at be this have from or one had by word but not what all were we when your can said there use an each which she do how their if will up other about out many then them these so some her would make like him into time has look two more write go see number no way could people my than first water been call who now find long down day did get come made may part over new sound take only little work know place year live back give most very after thing our just name good man think say great where help through much before line right too mean old any same tell follow came want show also around form three small set put end does another well large must big even such because turn here why ask went men read need land different home move try kind hand picture again change off play spell air away animal house point page letter mother answer found study still learn should world high every near add food between own below country plant last school father keep tree never start city earth eye light thought head under story saw left few while along might close something seem next hard open example begin life always those both paper together got group often run important until children side feet car mile night walk white sea began grow took river four carry state once book hear stop without second later miss idea enough eat face watch far real almost let above girl sometimes mountain cut young talk soon list song being leave family quick jump zone vex quiz box jazz fix query value buffer object";

pub const CODE: [&str; 10] = [
    "def fizz(n):\n    for i in range(1, n + 1):\n        if i % 15 == 0:\n            print(\"FizzBuzz\")\n        elif i % 3 == 0:\n            print(\"Fizz\")",
    "const total = items\n  .filter((x) => x.price > 0)\n  .reduce((sum, x) => sum + x.price * x.qty, 0);",
    "func Max(a, b int) int {\n    if a > b {\n        return a\n    }\n    return b\n}",
    "fn main() {\n    let v: Vec<i32> = (1..=5).map(|x| x * x).collect();\n    println!(\"{:?}\", v);\n}",
    "for f in *.log; do\n  grep -c \"ERROR\" \"$f\" && echo \"$f: done\"\ndone",
    "type User = { id: number; name: string };\nexport async function getUser(id: number): Promise<User> {\n  const res = await fetch(`/api/users/${id}`);\n  return res.json();\n}",
    "SELECT name, COUNT(*) AS n\nFROM orders o JOIN users u ON u.id = o.user_id\nWHERE o.created_at >= '2026-01-01'\nGROUP BY name ORDER BY n DESC;",
    "git switch -c feat/login && git add -p\ngit commit -m \"add login form\"\ngit push -u origin HEAD",
    "if err != nil {\n    return fmt.Errorf(\"load %s: %w\", path, err)\n}",
    "users = {u[\"id\"]: u for u in rows if u.get(\"active\")}\nprint(len(users), sorted(users)[:3])",
];

const fn c(deck: Deck, action: &'static str, combo: &'static str) -> Card {
    Card {
        deck,
        action,
        combo,
        recall: false,
    }
}
// recall-only: macOS or the terminal itself takes these keys first
const fn r(deck: Deck, action: &'static str, combo: &'static str) -> Card {
    Card {
        deck,
        action,
        combo,
        recall: true,
    }
}
use Deck::*;
pub const CARDS: &[Card] = &[
    c(Text, "Jump to start of line", "cmd+Left"),
    c(Text, "Jump to end of line", "cmd+Right"),
    c(Text, "Move back one word", "opt+Left"),
    c(Text, "Move forward one word", "opt+Right"),
    c(Text, "Jump to top of document", "cmd+Up"),
    c(Text, "Select to start of line", "cmd+shift+Left"),
    c(Text, "Select the next word", "opt+shift+Right"),
    c(Text, "Delete previous word", "opt+Back"),
    c(Text, "Delete to start of line", "cmd+Back"),
    c(Text, "Redo", "cmd+shift+Z"),
    c(
        Text,
        "Start of line, Emacs style (most Mac text fields)",
        "ctrl+A",
    ),
    c(Text, "Delete to end of line, Emacs style", "ctrl+K"),
    r(Text, "Forward delete one character", "fn+Back"),
    r(Mac, "Open Spotlight", "cmd+Space"),
    r(Mac, "Switch apps", "cmd+Tab"),
    c(Mac, "Cycle windows of the current app", "cmd+`"),
    r(Mac, "Hide current app", "cmd+H"),
    r(Mac, "Hide all other apps", "cmd+opt+H"),
    r(Mac, "Quit app", "cmd+Q"),
    r(Mac, "Close window", "cmd+W"),
    r(Mac, "Minimize window", "cmd+M"),
    r(Mac, "Open settings of the current app", "cmd+,"),
    r(Mac, "Screenshot an area", "cmd+shift+4"),
    r(Mac, "Screenshot and screen recording tools", "cmd+shift+5"),
    r(Mac, "Lock screen", "ctrl+cmd+Q"),
    r(Mac, "Force quit dialog", "cmd+opt+Esc"),
    r(Mac, "Mission Control", "ctrl+Up"),
    r(Mac, "Move to the desktop space on the right", "ctrl+Right"),
    r(Mac, "Emoji and symbols picker", "ctrl+cmd+Space"),
    c(Finder, "Go to folder by typing a path", "cmd+shift+G"),
    c(Finder, "Show or hide hidden files", "cmd+shift+."),
    r(Finder, "Quick Look the selected file", "Space"),
    c(Finder, "Open the parent folder", "cmd+Up"),
    c(Finder, "Open the selected item", "cmd+Down"),
    r(Finder, "New folder", "cmd+shift+N"),
    c(Finder, "Copy path of the selected file", "cmd+opt+C"),
    c(Finder, "Move file to Trash", "cmd+Back"),
    c(
        Finder,
        "Move a copied file here (cut and paste)",
        "cmd+opt+V",
    ),
    c(Zsh, "Start of command line", "ctrl+A"),
    c(Zsh, "End of command line", "ctrl+E"),
    c(Zsh, "Delete previous word", "ctrl+W"),
    c(Zsh, "Delete whole line", "ctrl+U"),
    c(Zsh, "Delete to end of line", "ctrl+K"),
    c(Zsh, "Search command history", "ctrl+R"),
    c(Zsh, "Clear the screen", "ctrl+L"),
    c(Zsh, "Paste back what you just deleted", "ctrl+Y"),
    c(Zsh, "Cancel the running command", "ctrl+C"),
    c(Zsh, "Previous command", "ctrl+P"),
    r(Idea, "Search everywhere", "shift shift"),
    c(Idea, "Find action by name", "cmd+shift+A"),
    c(Idea, "Recent files", "cmd+E"),
    c(Idea, "Go to declaration", "cmd+B"),
    c(Idea, "Show quick fixes", "opt+Enter"),
    c(Idea, "Reformat code", "cmd+opt+L"),
    c(Idea, "Rename symbol", "shift+F6"),
    c(Idea, "Duplicate line", "cmd+D"),
    c(Idea, "Delete line", "cmd+Back"),
    c(Idea, "Expand selection", "opt+Up"),
    c(Idea, "Toggle line comment", "cmd+/"),
    c(Idea, "Go to file", "cmd+shift+O"),
    c(Idea, "Extract variable", "cmd+opt+V"),
    c(Idea, "File structure popup", "cmd+F12"),
    c(Idea, "Find in files", "cmd+shift+F"),
    c(Idea, "Run", "ctrl+R"),
];

pub fn words() -> Vec<&'static str> {
    WORDS.split(' ').collect()
}

pub fn baseline_text(rng: &mut Rng, n: usize) -> String {
    let w = words();
    (0..n).map(|_| *rng.pick(&w)).collect::<Vec<_>>().join(" ")
}

// words weighted by how many focus letters they contain
pub fn weak_text(rng: &mut Rng, focus: &[char], n: usize) -> String {
    let mut pool = Vec::new();
    for w in words() {
        let hits = w.chars().filter(|ch| focus.contains(ch)).count();
        pool.extend(std::iter::repeat_n(w, 1 + hits * 4));
    }
    (0..n)
        .map(|_| *rng.pick(&pool))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn code_text(rng: &mut Rng) -> &'static str {
    rng.pick(&CODE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::parse;

    #[test]
    fn every_live_card_parses_and_every_unparseable_card_is_recall() {
        for card in CARDS.iter() {
            if parse(card.combo).is_none() {
                assert!(card.recall, "{} needs to be a recall card", card.combo);
            }
        }
    }

    #[test]
    fn card_ids_are_unique() {
        let mut ids: Vec<String> = CARDS.iter().map(|c| c.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CARDS.len());
    }

    #[test]
    fn baseline_has_n_words_from_the_list() {
        let t = baseline_text(&mut Rng::new(1), 40);
        let w = words();
        assert_eq!(t.split(' ').count(), 40);
        assert!(t.split(' ').all(|x| w.contains(&x)));
    }

    #[test]
    fn weak_text_leans_on_the_focus_letters() {
        let focus = ['q', 'z', 'x'];
        let hits = |t: &str| {
            t.split(' ')
                .filter(|w| w.chars().any(|ch| focus.contains(&ch)))
                .count()
        };
        let weak = weak_text(&mut Rng::new(9), &focus, 200);
        let base = baseline_text(&mut Rng::new(9), 200);
        assert_eq!(weak.split(' ').count(), 200);
        assert!(
            hits(&weak) > hits(&base) * 3,
            "weak {} base {}",
            hits(&weak),
            hits(&base)
        );
    }
}
