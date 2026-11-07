// mac us keyboard, each key tinted by the finger that should press it.
// the next key is lit, and capitals light the shift on the other hand.
use crate::app::App;
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// finger index: 0 left pinky .. 3 left index, 4 right index .. 7 right pinky, 8 thumbs
pub const FINGERS: [(&str, Color); 9] = [
    ("L pinky", Color::Rgb(243, 196, 208)),
    ("L ring", Color::Rgb(246, 215, 168)),
    ("L middle", Color::Rgb(207, 230, 176)),
    ("L index", Color::Rgb(185, 220, 240)),
    ("R index", Color::Rgb(201, 196, 242)),
    ("R middle", Color::Rgb(181, 227, 214)),
    ("R ring", Color::Rgb(242, 207, 233)),
    ("R pinky", Color::Rgb(227, 214, 194)),
    ("Thumbs", Color::Rgb(221, 225, 232)),
];
const PLAIN_KEY: Color = Color::Rgb(70, 74, 84);

struct K {
    label: String,
    // the char this key types unshifted, '\n' for return, ' ' for space
    ch: Option<char>,
    finger: usize,
    width: usize,
    shift_side: Option<bool>, // Some(true) = left shift
}

fn finger_of(code: u8) -> usize {
    (code - b'a') as usize
}

fn row(pre: Vec<K>, base: &str, fin: &str, post: Vec<K>) -> Vec<K> {
    let mut keys = pre;
    for (c, f) in base.chars().zip(fin.bytes()) {
        keys.push(K {
            label: c.to_ascii_uppercase().to_string(),
            ch: Some(c),
            finger: finger_of(f),
            width: 4,
            shift_side: None,
        });
    }
    keys.extend(post);
    keys
}

fn special(label: &str, ch: Option<char>, finger: usize, width: usize) -> K {
    K {
        label: label.into(),
        ch,
        finger,
        width,
        shift_side: None,
    }
}

fn layout() -> Vec<Vec<K>> {
    let shift = |left: bool| K {
        label: "shift".into(),
        ch: None,
        finger: if left { 0 } else { 7 },
        width: 9,
        shift_side: Some(left),
    };
    vec![
        row(
            vec![],
            "`1234567890-=",
            "aabcddeefghhh",
            vec![special("delete", None, 7, 6)],
        ),
        row(
            vec![special("tab", None, 0, 6)],
            "qwertyuiop[]\\",
            "abcddeefghhhh",
            vec![],
        ),
        row(
            vec![special("caps", None, 0, 7)],
            "asdfghjkl;'",
            "abcddeefghh",
            vec![special("return", Some('\n'), 7, 7)],
        ),
        row(
            vec![shift(true)],
            "zxcvbnm,./",
            "abcddeefgh",
            vec![shift(false)],
        ),
        vec![
            special("fn", None, 0, 4),
            special("ctrl", None, 0, 4),
            special("opt", None, 0, 4),
            special("cmd", None, 8, 5),
            special("space", Some(' '), 8, 24),
            special("cmd", None, 8, 5),
            special("opt", None, 7, 4),
        ],
    ]
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

fn mix(a: Color, b: Color, t: f64) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let l = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
            Color::Rgb(l(r1, r2), l(g1, g2), l(b1, b2))
        }
        _ => a,
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
    let want_finger = layout()
        .iter()
        .flatten()
        .find(|k| k.ch.is_some() && k.ch == want)
        .map(|k| k.finger);
    let avg = avg_ms(&app.progress.stats);
    let mut lines: Vec<Line> = layout()
        .iter()
        .map(|keys| {
            let mut spans = Vec::new();
            for k in keys {
                let mut bg = if app.fingers {
                    FINGERS[k.finger].1
                } else {
                    PLAIN_KEY
                };
                let mut fg = if app.fingers {
                    Color::Black
                } else {
                    Color::White
                };
                if let (true, Some(c)) = (app.heat, k.ch)
                    && let Some(s) = app.progress.stats.get(&c).filter(|s| s.n >= MIN_SAMPLES)
                {
                    let t = ((weakness(Some(s), avg) - 0.8) / 1.5).clamp(0.0, 0.85);
                    bg = mix(bg, super::BAD, t);
                }
                // shift on the opposite hand from the key it modifies
                let lit_shift =
                    needs_shift && k.shift_side == Some(want_finger.is_some_and(|f| f >= 4));
                let mut style = Style::new();
                if (k.ch.is_some() && k.ch == want) || lit_shift {
                    bg = super::ACCENT;
                    fg = Color::Black;
                    style = style.add_modifier(Modifier::BOLD);
                }
                let label = if app.blind {
                    String::new()
                } else {
                    k.label.clone()
                };
                spans.push(Span::styled(
                    format!("{label:^w$}", w = k.width),
                    style.bg(bg).fg(fg),
                ));
                spans.push(Span::raw(" "));
            }
            Line::from(spans)
        })
        .collect();
    let legend: Vec<Span> = FINGERS
        .iter()
        .flat_map(|(name, c)| {
            [
                Span::styled("  ", Style::new().bg(*c)),
                Span::styled(format!(" {name}  "), Style::new().fg(super::DIM)),
            ]
        })
        .collect();
    lines.push(Line::from(legend));
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}
