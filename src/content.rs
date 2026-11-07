// practice material and the builders that turn it into drill text
// (the built-in shortcut cards live in decks.rs)
pub use crate::decks::CARDS;
use crate::rng::Rng;
use std::collections::BTreeMap;

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

pub fn words() -> Vec<&'static str> {
    WORDS.split(' ').collect()
}

// words weighted by how many focus letters and weak pairs they contain
pub fn weak_text(rng: &mut Rng, focus: &[char], pairs: &[String], n: usize) -> String {
    let mut pool = Vec::new();
    for w in words() {
        let hits = w.chars().filter(|ch| focus.contains(ch)).count();
        let pair_hits: usize = pairs.iter().map(|p| w.matches(p.as_str()).count()).sum();
        pool.extend(std::iter::repeat_n(w, 1 + hits * 4 + pair_hits * 6));
    }
    (0..n)
        .map(|_| *rng.pick(&pool))
        .collect::<Vec<_>>()
        .join(" ")
}

// words you missed, weighted by how often (capped so one word cannot fill
// the whole round); empty when there is nothing to practice yet
pub fn mistakes_text(rng: &mut Rng, mistakes: &BTreeMap<String, u32>, n: usize) -> String {
    let mut pool: Vec<&str> = Vec::new();
    for (w, count) in mistakes {
        pool.extend(std::iter::repeat_n(w.as_str(), (*count).min(5) as usize));
    }
    if pool.is_empty() {
        return String::new();
    }
    (0..n)
        .map(|_| *rng.pick(&pool))
        .collect::<Vec<_>>()
        .join(" ")
}

// your own file split into rounds: up to 6 non-blank lines each, tabs as
// 4 spaces, control characters removed (a file must never be able to send
// escape codes to the terminal), lines with untypeable characters skipped,
// and the indentation the lines share removed
pub fn own_text(raw: &str) -> Vec<String> {
    let lines: Vec<String> = raw
        .lines()
        .map(|l| {
            l.replace('\t', "    ")
                .chars()
                .filter(|c| !c.is_control())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .filter(|l| !l.trim().is_empty() && l.is_ascii())
        .collect();
    lines
        .chunks(OWN_LINES)
        .map(|chunk| {
            let indent = chunk
                .iter()
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            chunk
                .iter()
                .map(|l| &l[indent..])
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

// lines per round when typing your own file
const OWN_LINES: usize = 6;

pub fn code_text(rng: &mut Rng) -> &'static str {
    rng.pick(&CODE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_text_leans_on_the_focus_letters() {
        let focus = ['q', 'z', 'x'];
        let hits = |t: &str| {
            t.split(' ')
                .filter(|w| w.chars().any(|ch| focus.contains(&ch)))
                .count()
        };
        let weak = weak_text(&mut Rng::new(9), &focus, &[], 200);
        let base = crate::baseline::text(
            &mut Rng::new(9),
            crate::baseline::Length::Words(200),
            false,
            false,
        );
        assert_eq!(weak.split(' ').count(), 200);
        assert!(
            hits(&weak) > hits(&base) * 3,
            "weak {} base {}",
            hits(&weak),
            hits(&base)
        );
    }

    #[test]
    fn own_text_is_cleaned_before_it_reaches_the_screen() {
        let raw = "fn main() {\n\tlet x = 1;\u{1b}[2J\n}\n";
        let chunks = own_text(raw);
        assert_eq!(
            chunks,
            vec!["fn main() {\n    let x = 1;[2J\n}".to_string()]
        );
        assert!(
            chunks
                .iter()
                .all(|c| !c.chars().any(|ch| ch.is_control() && ch != '\n'))
        );
    }

    #[test]
    fn own_text_skips_lines_you_cannot_type_and_splits_long_files() {
        let mut raw = String::from("caf\u{e9} au lait\n");
        for i in 0..20 {
            raw.push_str(&format!("let value_{i} = compute({i});\n"));
        }
        let chunks = own_text(&raw);
        assert!(chunks.iter().all(|c| c.lines().count() <= 6));
        assert!(
            chunks.iter().all(|c| c.is_ascii()),
            "lines with accents are skipped"
        );
        assert_eq!(chunks.iter().map(|c| c.lines().count()).sum::<usize>(), 20);
    }

    #[test]
    fn own_text_removes_shared_indentation() {
        let chunks = own_text("        if ready {\n            go();\n        }\n");
        assert_eq!(chunks, vec!["if ready {\n    go();\n}".to_string()]);
    }

    #[test]
    fn own_text_of_an_empty_file_is_empty() {
        assert!(own_text("\n\n   \n").is_empty());
    }

    #[test]
    fn weak_text_also_leans_on_weak_pairs() {
        let pairs = vec!["ng".to_string()];
        let hits = |t: &str| t.split(' ').filter(|w| w.contains("ng")).count();
        let with = weak_text(&mut Rng::new(4), &[], &pairs, 300);
        let without = weak_text(&mut Rng::new(4), &[], &[], 300);
        assert!(
            hits(&with) > hits(&without) * 2,
            "with {} without {}",
            hits(&with),
            hits(&without)
        );
    }
}
