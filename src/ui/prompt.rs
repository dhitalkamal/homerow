// the text you type: typed chars fade, the cursor is an accent block, a miss
// is red, the rest is bright. long text scrolls so the cursor line stays near the top.
use crate::app::App;
use crate::session::Cell;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// splits text into [start, end) line ranges no wider than width. a newline ends
// its line (and belongs to it); long lines break after the last space that fits.
pub fn lines(text: &[char], width: usize) -> Vec<(usize, usize)> {
    let (len, width) = (text.len(), width.max(1));
    let mut out = Vec::new();
    let mut start = 0;
    while start < len {
        let limit = (start + width).min(len);
        if let Some(nl) = (start..limit).find(|&i| text[i] == '\n') {
            out.push((start, nl + 1));
            start = nl + 1;
            continue;
        }
        if limit == len {
            out.push((start, len));
            break;
        }
        // keep the space on the line it ends, like an editor does
        let end = (start..limit)
            .rev()
            .find(|&i| text[i] == ' ')
            .map_or(limit, |sp| sp + 1);
        out.push((start, end));
        start = end;
    }
    out
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let s = &app.session;
    let ranges = lines(&s.text, area.width.max(10) as usize);
    let cur = ranges
        .iter()
        .position(|&(a, b)| s.pos() >= a && s.pos() < b)
        .unwrap_or(0);
    let first = cur.saturating_sub(1);
    let return_glyph = char::from_u32(0x23CE).unwrap_or('|').to_string();
    // a blank row between text rows, so each row takes two lines
    let rows = (area.height as usize).div_ceil(2);
    let out: Vec<Line> = ranges
        .iter()
        .skip(first)
        .take(rows)
        .flat_map(|&(a, b)| {
            let text = Line::from(
                (a..b)
                    .map(|i| {
                        let ch = s.text[i];
                        let style = match s.cell(i) {
                            // typed text fades, what you still have to read stays bright
                            Cell::Done => Style::new().add_modifier(Modifier::DIM),
                            Cell::Current => Style::new()
                                .bg(super::ACCENT)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                            Cell::Missed => Style::new().bg(super::BAD).fg(Color::Black),
                            Cell::Pending => Style::new(),
                        };
                        let shown = if ch == '\n' {
                            return_glyph.clone()
                        } else {
                            ch.to_string()
                        };
                        Span::styled(shown, style)
                    })
                    .collect::<Vec<_>>(),
            );
            [text, Line::raw("")]
        })
        .collect();
    f.render_widget(Paragraph::new(out), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(s: &str, w: usize) -> Vec<String> {
        let t: Vec<char> = s.chars().collect();
        lines(&t, w)
            .into_iter()
            .map(|(a, b)| t[a..b].iter().collect())
            .collect()
    }

    #[test]
    fn short_text_is_one_line() {
        assert_eq!(split("the cat", 20), vec!["the cat"]);
    }

    #[test]
    fn wraps_after_the_last_space_that_fits() {
        assert_eq!(
            split("the quick brown fox", 10),
            vec!["the quick ", "brown fox"]
        );
    }

    #[test]
    fn newline_ends_a_line_and_stays_with_it() {
        assert_eq!(split("a {\n  b\n}", 20), vec!["a {\n", "  b\n", "}"]);
    }

    #[test]
    fn a_word_longer_than_the_width_is_hard_broken() {
        assert_eq!(split("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn ranges_cover_every_char_exactly_once() {
        let t: Vec<char> = "one two three\nfour five six seven eight".chars().collect();
        let r = lines(&t, 7);
        assert_eq!(r.first().unwrap().0, 0);
        assert_eq!(r.last().unwrap().1, t.len());
        assert!(r.windows(2).all(|w| w[0].1 == w[1].0));
        assert!(r.iter().all(|&(a, b)| b - a <= 7));
    }
}
