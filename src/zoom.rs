// kitty text sizing (OSC 66): draws text at 2x or 3x inside kitty without
// changing the terminal font. this module is pure: it decides support from
// cursor positions and turns zoom runs into escape codes. main.rs writes them.
use ratatui::style::{Color, Modifier, Style};

// kitty limits the text inside one escape code to 4096 bytes
const MAX_PAYLOAD: usize = 4096;

// one piece of zoomed text: where it starts (in cells), how big, and its style
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub x: u16,
    pub y: u16,
    pub scale: u8,
    pub text: String,
    pub style: Style,
}

// the probe writes a 2x space; a terminal that understands it moves 2 cells
pub fn supported(before_col: u16, after_col: u16) -> bool {
    after_col == before_col + 2
}

// control characters (esc, bel, newline) would end or break the escape code
pub fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

// select graphic rendition for a style, always starting from a reset
pub fn sgr(style: Style) -> String {
    let mut codes = vec!["0".to_string()];
    if style.add_modifier.contains(Modifier::BOLD) {
        codes.push("1".into());
    }
    if style.add_modifier.contains(Modifier::DIM) {
        codes.push("2".into());
    }
    codes.extend(style.fg.and_then(|c| color_code(c, true)));
    codes.extend(style.bg.and_then(|c| color_code(c, false)));
    format!("\u{1b}[{}m", codes.join(";"))
}

// sgr parameter for a color; None leaves the terminal default
fn color_code(c: Color, fg: bool) -> Option<String> {
    let base = if fg { 30 } else { 40 };
    let named = |n: u8| Some((base + n).to_string());
    let bright = |n: u8| Some((base + 60 + n).to_string());
    match c {
        Color::Reset => None,
        Color::Black => named(0),
        Color::Red => named(1),
        Color::Green => named(2),
        Color::Yellow => named(3),
        Color::Blue => named(4),
        Color::Magenta => named(5),
        Color::Cyan => named(6),
        Color::Gray => named(7),
        Color::DarkGray => bright(0),
        Color::LightRed => bright(1),
        Color::LightGreen => bright(2),
        Color::LightYellow => bright(3),
        Color::LightBlue => bright(4),
        Color::LightMagenta => bright(5),
        Color::LightCyan => bright(6),
        Color::White => bright(7),
        Color::Rgb(r, g, b) => Some(format!("{};2;{r};{g};{b}", base + 8)),
        Color::Indexed(i) => Some(format!("{};5;{i}", base + 8)),
    }
}

// split on char boundaries so no piece is over the payload limit
fn chunks(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if i + c.len_utf8() - start > MAX_PAYLOAD {
            out.push(&text[start..i]);
            start = i;
        }
    }
    out.push(&text[start..]);
    out
}

// escape codes for all runs, ending with a style reset
pub fn encode(runs: &[Run]) -> String {
    let mut out = String::new();
    for r in runs {
        // cursor position is 1-based, row first
        out.push_str(&format!("\u{1b}[{};{}H", r.y + 1, r.x + 1));
        out.push_str(&sgr(r.style));
        let clean = sanitize(&r.text);
        // each piece leaves the cursor after itself, so pieces follow on
        for piece in chunks(&clean) {
            out.push_str(&format!("\u{1b}]66;s={};{piece}\u{7}", r.scale));
        }
    }
    out.push_str("\u{1b}[0m");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(x: u16, y: u16, text: &str, style: Style) -> Run {
        Run {
            x,
            y,
            scale: 2,
            text: text.into(),
            style,
        }
    }

    #[test]
    fn supported_only_when_the_cursor_moved_two_cells() {
        assert!(supported(0, 2));
        assert!(supported(5, 7));
        assert!(!supported(0, 0), "terminal ignored the code");
        assert!(!supported(0, 1), "terminal printed a plain space");
    }

    #[test]
    fn sanitize_replaces_control_characters_with_spaces() {
        assert_eq!(sanitize("a\u{1b}b\u{7}c\nd\u{7f}"), "a b c d ");
        assert_eq!(sanitize("fn main() {}"), "fn main() {}");
    }

    #[test]
    fn sgr_covers_the_styles_the_prompt_uses() {
        assert_eq!(sgr(Style::new()), "\u{1b}[0m");
        let cursor = Style::new()
            .fg(Color::Black)
            .bg(Color::Rgb(255, 196, 92))
            .add_modifier(Modifier::BOLD);
        assert_eq!(sgr(cursor), "\u{1b}[0;1;30;48;2;255;196;92m");
        assert_eq!(sgr(Style::new().add_modifier(Modifier::DIM)), "\u{1b}[0;2m");
        assert_eq!(
            sgr(Style::new().bg(Color::Red).fg(Color::Black)),
            "\u{1b}[0;30;41m"
        );
    }

    #[test]
    fn encode_moves_styles_and_scales_each_run() {
        let out = encode(&[run(4, 2, "hi", Style::new())]);
        assert_eq!(out, "\u{1b}[3;5H\u{1b}[0m\u{1b}]66;s=2;hi\u{7}\u{1b}[0m");
    }

    #[test]
    fn encode_sanitizes_the_text() {
        let out = encode(&[run(0, 0, "a\u{7}b", Style::new())]);
        assert!(out.contains("]66;s=2;a b\u{7}"), "{out:?}");
    }

    #[test]
    fn long_text_is_split_under_the_payload_limit() {
        let long = "x".repeat(MAX_PAYLOAD + 10);
        let out = encode(&[run(0, 0, &long, Style::new())]);
        let payloads: Vec<&str> = out
            .split("]66;s=2;")
            .skip(1)
            .map(|p| p.split('\u{7}').next().unwrap())
            .collect();
        assert_eq!(payloads.len(), 2);
        assert!(payloads.iter().all(|p| p.len() <= MAX_PAYLOAD));
        assert_eq!(payloads.concat().len(), long.len());
    }
}
