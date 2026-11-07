// two hands drawn as finger bars under the keyboard. the finger for the next
// key is lit in its color, the rest stay dim.
use super::keyboard::FINGERS;
use super::{ACCENT, KEYCAP, dim};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// 5 rows of bars, a row of finger letters, then the hand captions
pub const HEIGHT: u16 = 7;
const BAR_ROWS: u16 = 5;
const BAR: &str = "\u{2588}\u{2588}";

// bar height per finger index (see FINGERS), middle fingers tallest
const TALL: [u16; 9] = [3, 4, 5, 4, 4, 5, 4, 3, 2];
const THUMB: usize = 8;
const LEFT: [usize; 4] = [0, 1, 2, 3];
const RIGHT: [usize; 4] = [4, 5, 6, 7];
// letters under each finger, in screen order
const LEFT_TAGS: &str = "p  r  m  i     t";
const RIGHT_TAGS: &str = "t     i  m  r  p";
const GAP: &str = "        ";

pub fn render(f: &mut Frame, area: Rect, active: &[usize], colors: bool) {
    let color = |finger: usize| -> Color {
        match (active.contains(&finger), colors) {
            (true, true) => FINGERS[finger].1,
            (true, false) => ACCENT,
            (false, _) => KEYCAP,
        }
    };
    let cell = |finger: usize, row: u16| -> Span<'static> {
        let filled = row >= BAR_ROWS - TALL[finger];
        let text = if filled { BAR } else { "  " };
        Span::styled(format!("{text} "), Style::new().fg(color(finger)))
    };
    let mut lines = Vec::new();
    for row in 0..BAR_ROWS {
        let mut spans: Vec<Span> = LEFT.iter().map(|&f| cell(f, row)).collect();
        spans.push(Span::raw("  "));
        spans.push(cell(THUMB, row));
        spans.push(Span::raw(GAP));
        spans.push(cell(THUMB, row));
        spans.push(Span::raw("  "));
        spans.extend(RIGHT.iter().map(|&f| cell(f, row)));
        lines.push(Line::from(spans));
    }
    lines.push(Line::styled(
        format!("{LEFT_TAGS}  {GAP}{RIGHT_TAGS}  "),
        dim(),
    ));
    lines.push(Line::styled(
        format!("{:^17}{GAP}{:^17}", "left hand", "right hand"),
        dim(),
    ));
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}
