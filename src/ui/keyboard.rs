// mac us keyboard drawn as keycaps. the letter on each key is tinted by the
// finger that should press it, the next key fills with that finger's color,
// and capitals light the shift on the other hand. one hint line underneath
// names the finger in words.
use crate::app::App;
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// full: 5 key rows with a blank line between, a blank line, then the hint.
// compact: the same without the blank lines between rows.
pub const FULL_HEIGHT: u16 = 11;
pub const COMPACT_HEIGHT: u16 = 7;
// every row, counting the one column gap after each key
#[cfg(test)]
const ROW_WIDTH: usize = 60;

// finger index: 0 left pinky .. 3 left index, 4 right index .. 7 right pinky, 8 thumbs.
// fixed hues spread around the color wheel. ansi colors were tried first, but
// many themes draw "bright" ansi the same as normal, so fingers shared colors.
pub const FINGERS: [(&str, Color); 9] = [
    ("left pinky", Color::Rgb(199, 146, 234)),
    ("left ring", Color::Rgb(130, 170, 255)),
    ("left middle", Color::Rgb(137, 221, 255)),
    ("left index", Color::Rgb(195, 232, 141)),
    ("right index", Color::Rgb(255, 203, 107)),
    ("right middle", Color::Rgb(247, 140, 108)),
    ("right ring", Color::Rgb(255, 83, 112)),
    ("right pinky", Color::Rgb(78, 201, 176)),
    ("thumbs", Color::Rgb(160, 168, 196)),
];

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

fn layout() -> Vec<Vec<K>> {
    let shift = |left: bool| K {
        label: "shift",
        ch: None,
        finger: if left { 0 } else { 7 },
        width: 9,
        shift_side: Some(left),
    };
    let mut r0 = row("`1234567890-=", "aabcddeefghhh");
    r0.push(key("del", None, 7, 7));
    let mut r1 = vec![key("tab", None, 0, 7)];
    r1.extend(row("qwertyuiop[]\\", "abcddeefghhhh"));
    let mut r2 = vec![key("caps", None, 0, 6)];
    r2.extend(row("asdfghjkl;'", "abcddeefghh"));
    r2.push(key("return", Some('\n'), 7, 8));
    let mut r3 = vec![shift(true)];
    r3.extend(row("zxcvbnm,./", "abcddeefgh"));
    r3.push(shift(false));
    let r4 = vec![
        key("fn", None, 0, 4),
        key("ctrl", None, 0, 6),
        key("opt", None, 0, 5),
        key("cmd", None, 8, 5),
        key("", Some(' '), 8, 23),
        key("cmd", None, 8, 5),
        key("opt", None, 7, 5),
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

// which fingers press this char: the key's finger, plus the pinky on the
// other hand when it needs shift. space lights the thumbs.
pub fn fingers_for(c: char) -> Vec<usize> {
    let (k, shift) = base_key(c);
    let Some(finger) = layout()
        .iter()
        .flatten()
        .find(|key| key.ch == Some(k))
        .map(|key| key.finger)
    else {
        return Vec::new();
    };
    let mut out = vec![finger];
    if shift {
        out.push(if finger >= 4 { 0 } else { 7 });
    }
    out
}

// next is the char to light up; the settings preview passes a sample one
pub fn render(f: &mut Frame, area: Rect, app: &App, compact: bool, next: Option<char>) {
    let opts = app.progress.settings;
    let (want, needs_shift) = next.map_or((None, false), |c| {
        let (k, s) = base_key(c);
        (Some(k), s)
    });
    let keys = layout();
    let target = keys
        .iter()
        .flatten()
        .find(|k| k.ch.is_some() && k.ch == want);
    let avg = avg_ms(&app.progress.stats);
    let mut lines = Vec::new();
    for (i, row) in keys.iter().enumerate() {
        if i > 0 && !compact {
            lines.push(Line::raw(""));
        }
        let mut spans = Vec::new();
        for k in row {
            let color = FINGERS[k.finger].1;
            let mut style = Style::new().bg(super::KEYCAP);
            if opts.finger_colors {
                style = style.fg(color);
            }
            if let (true, Some(c)) = (opts.heatmap, k.ch)
                && let Some(s) = app.progress.stats.get(&c).filter(|s| s.n >= MIN_SAMPLES)
                && weakness(Some(s), avg) > 1.4
            {
                style = style
                    .fg(super::BAD)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            }
            // shift goes on the opposite hand from the key it modifies
            let lit_shift =
                needs_shift && k.shift_side == Some(target.is_some_and(|t| t.finger >= 4));
            if target.is_some_and(|t| std::ptr::eq(t, k)) || lit_shift {
                style = Style::new()
                    .bg(color)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }
            let label = if opts.hide_labels { "" } else { k.label };
            spans.push(Span::styled(format!("{label:^w$}", w = k.width), style));
            spans.push(Span::raw(" "));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::raw(""));
    if let Some(t) = target {
        let (name, color) = FINGERS[t.finger];
        let label = match t.ch {
            Some(' ') => "space",
            _ => t.label,
        };
        let mut hint = vec![
            Span::styled("next  ", super::dim()),
            Span::styled(
                format!(" {label} "),
                Style::new()
                    .bg(color)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {name}"), Style::new().fg(color)),
        ];
        if needs_shift {
            let side = if t.finger >= 4 { "left" } else { "right" };
            hint.push(Span::styled(format!("   with {side} shift"), super::dim()));
        }
        lines.push(Line::from(hint));
    }
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_equal_width_and_labels_have_room() {
        for keys in layout() {
            let w: usize = keys.iter().map(|k| k.width + 1).sum();
            assert_eq!(w, ROW_WIDTH);
            for k in &keys {
                assert!(k.width >= k.label.len() + 2, "{:?} is cramped", k.label);
            }
        }
    }

    #[test]
    fn fingers_for_lights_the_key_finger_and_the_other_pinky_for_shift() {
        assert_eq!(fingers_for('a'), vec![0]);
        assert_eq!(fingers_for('j'), vec![4]);
        assert_eq!(fingers_for('A'), vec![0, 7]);
        assert_eq!(fingers_for('J'), vec![4, 0]);
        assert_eq!(fingers_for('?'), vec![7, 0]);
        assert_eq!(fingers_for(' '), vec![8]);
        assert_eq!(fingers_for('\u{e9}'), Vec::<usize>::new());
    }

    #[test]
    fn finger_colors_are_fixed_and_distinct() {
        let cs: Vec<Color> = FINGERS.iter().map(|f| f.1).collect();
        assert!(
            cs.iter().all(|c| matches!(c, Color::Rgb(..))),
            "a theme can merge ansi colors"
        );
        for (i, a) in cs.iter().enumerate() {
            assert!(cs[i + 1..].iter().all(|b| b != a));
        }
    }
}
