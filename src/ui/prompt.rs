// the text you type: typed chars fade, the cursor is an accent block, a miss
// is red, the rest is bright. long text scrolls so the cursor line stays near the top.
use crate::app::App;
use crate::session::Cell;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use tui_big_text::{BigText, PixelSize};

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

// typed text fades, what you still have to read stays bright
fn style_for(cell: Cell) -> Style {
    match cell {
        Cell::Done => Style::new().add_modifier(Modifier::DIM),
        Cell::Current => Style::new()
            .bg(super::ACCENT)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
        Cell::Missed => Style::new().bg(super::BAD).fg(Color::Black),
        Cell::Pending => Style::new(),
    }
}

// one text row as styled spans; newline shows as `newline`
fn styled_row(app: &App, a: usize, b: usize, newline: &str) -> Line<'static> {
    let s = &app.session;
    Line::from(
        (a..b)
            .map(|i| {
                let ch = s.text[i];
                let shown = if ch == '\n' {
                    newline.to_string()
                } else {
                    ch.to_string()
                };
                Span::styled(shown, style_for(s.cell(i)))
            })
            .collect::<Vec<_>>(),
    )
}

// the row ranges for this width and the index of the row holding the cursor
fn rows_and_cursor(app: &App, width: usize) -> (Vec<(usize, usize)>, usize) {
    let s = &app.session;
    let ranges = lines(&s.text, width);
    let cur = ranges
        .iter()
        .position(|&(a, b)| s.pos() >= a && s.pos() < b)
        .unwrap_or(0);
    (ranges, cur)
}

// spaced puts a blank line after each text row
pub fn render(f: &mut Frame, area: Rect, app: &App, spaced: bool) {
    let (ranges, cur) = rows_and_cursor(app, area.width.max(10) as usize);
    let return_glyph = char::from_u32(0x23CE).unwrap_or('|').to_string();
    let rows = if spaced {
        (area.height as usize).div_ceil(2)
    } else {
        area.height as usize
    };
    let out: Vec<Line> = ranges
        .iter()
        .skip(cur.saturating_sub(1))
        .take(rows)
        .flat_map(|&(a, b)| {
            let text = styled_row(app, a, b, &return_glyph);
            if spaced {
                vec![text, Line::raw("")]
            } else {
                vec![text]
            }
        })
        .collect();
    f.render_widget(Paragraph::new(out), area);
}

// block letter text that works in any terminal: each letter is 4 columns by
// 4 rows of quadrant characters, rows are 5 lines apart
pub fn render_big(f: &mut Frame, area: Rect, app: &App, rows: u16) {
    let letters = (area.width / 4).max(1) as usize;
    let (ranges, cur) = rows_and_cursor(app, letters);
    // with 3 or more rows keep the previous row in view, like normal text
    let first = if rows >= 3 {
        cur.saturating_sub(1)
    } else {
        cur
    };
    for (i, &(a, b)) in ranges.iter().skip(first).take(rows as usize).enumerate() {
        let y = area.y + i as u16 * 5;
        if y + 4 > area.y + area.height {
            break;
        }
        // the return glyph is not in the block font, a space keeps the cursor visible
        let big = BigText::builder()
            .pixel_size(PixelSize::Quadrant)
            .lines(vec![styled_row(app, a, b, " ")])
            .build();
        f.render_widget(big, Rect::new(area.x, y, area.width, 4));
    }
}

// kitty zoom: no cells are drawn here, the text comes back as runs (one per
// style change) that main.rs writes with the text sizing escape code. each row
// is padded with spaces to the full width so longer old text is overwritten.
pub fn zoom_runs(area: Rect, app: &App, rows: u16, scale: u16) -> Vec<crate::zoom::Run> {
    let s = &app.session;
    let letters = (area.width / scale).max(1) as usize;
    let (ranges, cur) = rows_and_cursor(app, letters);
    let first = if rows >= 3 {
        cur.saturating_sub(1)
    } else {
        cur
    };
    let mut runs = Vec::new();
    for row in 0..rows as usize {
        let y = area.y + row as u16 * (scale + 1);
        let mut x = area.x;
        let mut used = 0;
        if let Some(&(a, b)) = ranges.get(first + row) {
            let mut i = a;
            while i < b {
                let style = style_for(s.cell(i));
                let mut text = String::new();
                while i < b && style_for(s.cell(i)) == style {
                    text.push(if s.text[i] == '\n' { ' ' } else { s.text[i] });
                    i += 1;
                }
                let n = text.chars().count();
                runs.push(crate::zoom::Run {
                    x,
                    y,
                    scale: scale as u8,
                    text,
                    style,
                });
                x += n as u16 * scale;
                used += n;
            }
        }
        if used < letters {
            let text = " ".repeat(letters - used);
            runs.push(crate::zoom::Run {
                x,
                y,
                scale: scale as u8,
                text,
                style: Style::new(),
            });
        }
    }
    runs
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
