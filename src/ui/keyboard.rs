// mac us keyboard drawn as keycaps. each key has a colored edge for the finger
// that should press it. the next key fills with that finger's color, and
// capitals light the shift on the other hand.
use crate::app::App;
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// 5 rows of keycap + edge, a blank line, then the legend
pub const HEIGHT: u16 = 12;

// finger index: 0 left pinky .. 3 left index, 4 right index .. 7 right pinky, 8 thumbs.
// ansi colors, so your terminal theme decides the exact shades.
pub const FINGERS: [(&str, Color); 9] = [
    ("L pinky", Color::Magenta),
    ("L ring", Color::Blue),
    ("L middle", Color::Cyan),
    ("L index", Color::Green),
    ("R index", Color::Yellow),
    ("R middle", Color::Red),
    ("R ring", Color::LightMagenta),
    ("R pinky", Color::LightBlue),
    ("thumbs", Color::Gray),
];

const EDGE: &str = "\u{2594}";
const SWATCH: &str = "\u{2588}\u{2588}";

struct K {
    label: &'static str,
    // the char this key types unshifted, '\n' for return, ' ' for space
    ch: Option<char>,
    finger: usize,
    width: usize,
    // Some(true) = left shift
    shift_side: Option<bool>,
}

fn key(label: &'static str, ch: Option<char>, finger: usize, width: usize) -> K {
    K {
        label,
        ch,
        finger,
        width,
        shift_side: None,
    }
}

// labels for the char keys, uppercase like real keycaps
const LABELS: &str = "`1234567890-=QWERTYUIOP[]\\ASDFGHJKL;'ZXCVBNM,./";

fn row(chars: &'static str, fin: &str) -> Vec<K> {
    chars
        .chars()
        .zip(fin.bytes())
        .map(|(c, f)| {
            let at = LABELS.find(c.to_ascii_uppercase()).unwrap_or(0);
            key(&LABELS[at..at + 1], Some(c), (f - b'a') as usize, 3)
        })
        .collect()
}

// every row is 58 columns wide including the gap after each key
fn layout() -> Vec<Vec<K>> {
    let shift = |left: bool| K {
        label: "shift",
        ch: None,
        finger: if left { 0 } else { 7 },
        width: 8,
        shift_side: Some(left),
    };
    let mut r0 = row("`1234567890-=", "aabcddeefghhh");
    r0.push(key("del", None, 7, 5));
    let mut r1 = vec![key("tab", None, 0, 5)];
    r1.extend(row("qwertyuiop[]\\", "abcddeefghhhh"));
    let mut r2 = vec![key("caps", None, 0, 6)];
    r2.extend(row("asdfghjkl;'", "abcddeefghh"));
    r2.push(key("return", Some('\n'), 7, 6));
    let mut r3 = vec![shift(true)];
    r3.extend(row("zxcvbnm,./", "abcddeefgh"));
    r3.push(shift(false));
    let r4 = vec![
        key("fn", None, 0, 4),
        key("ctrl", None, 0, 4),
        key("opt", None, 0, 4),
        key("cmd", None, 8, 4),
        key("", Some(' '), 8, 27),
        key("cmd", None, 8, 4),
        key("opt", None, 7, 4),
    ];
    vec![r0, r1, r2, r3, r4]
}

const SHIFT_PAIRS: &str = "~`!1@2#3$4%5^6&7*8(9)0_-+={[}]|\\:;\"'<,>.?/";

// the key a char lives on, and whether it needs shift
fn base_key(c: char) -> (char, bool) {
    if c.is_ascii_uppercase() {
        return (c.to_ascii_lowercase(), true);
    }
    let pairs: Vec<char> = SHIFT_PAIRS.chars().collect();
    match pairs.chunks(2).find(|p| p[0] == c) {
        Some(p) => (p[1], true),
        None => (c, false),
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let next = if app.session.is_done() {
        None
    } else {
        app.session.next_char()
    };
    let (want, needs_shift) = next.map_or((None, false), |c| {
        let (k, s) = base_key(c);
        (Some(k), s)
    });
    let keys = layout();
    let want_finger = keys
        .iter()
        .flatten()
        .find(|k| k.ch.is_some() && k.ch == want)
        .map(|k| k.finger);
    let avg = avg_ms(&app.progress.stats);
    let mut lines = Vec::new();
    for keys in &keys {
        let (mut caps, mut edges) = (Vec::new(), Vec::new());
        for k in keys {
            let finger = FINGERS[k.finger].1;
            let mut edge = if app.fingers { finger } else { super::KEYCAP };
            let mut cap = Style::new().bg(super::KEYCAP);
            // weak keys: red label, very weak keys get a red edge too
            if let (true, Some(c)) = (app.heat, k.ch)
                && let Some(s) = app.progress.stats.get(&c).filter(|s| s.n >= MIN_SAMPLES)
            {
                let w = weakness(Some(s), avg);
                if w > 1.4 {
                    cap = cap.fg(super::BAD).add_modifier(Modifier::BOLD);
                }
                if w > 2.2 {
                    edge = super::BAD;
                }
            }
            // shift goes on the opposite hand from the key it modifies
            let lit_shift =
                needs_shift && k.shift_side == Some(want_finger.is_some_and(|f| f >= 4));
            if (k.ch.is_some() && k.ch == want) || lit_shift {
                cap = Style::new()
                    .bg(finger)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
                edge = finger;
            }
            let label = if app.blind { "" } else { k.label };
            caps.push(Span::styled(format!("{label:^w$}", w = k.width), cap));
            caps.push(Span::raw(" "));
            edges.push(Span::styled(EDGE.repeat(k.width), Style::new().fg(edge)));
            edges.push(Span::raw(" "));
        }
        lines.push(Line::from(caps));
        lines.push(Line::from(edges));
    }
    if app.fingers {
        lines.push(Line::raw(""));
        let legend: Vec<Span> = FINGERS
            .iter()
            .flat_map(|(name, c)| {
                [
                    Span::styled(SWATCH, Style::new().fg(*c)),
                    Span::styled(format!(" {name}   "), super::dim()),
                ]
            })
            .collect();
        lines.push(Line::from(legend));
    }
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}
