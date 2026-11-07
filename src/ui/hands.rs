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
const BLOCK: &str = "\u{2588}";

// bar height per finger index (see FINGERS), middle fingers tallest
const TALL: [u16; 9] = [3, 4, 5, 4, 4, 5, 4, 3, 2];
const THUMB: usize = 8;
const LEFT: [usize; 4] = [0, 1, 2, 3];
const RIGHT: [usize; 4] = [4, 5, 6, 7];
// letter under each finger index
const TAGS: [char; 9] = ['p', 'r', 'm', 'i', 'i', 'm', 'r', 'p', 't'];

// scale 1 draws bars 2 columns wide, scale 2 draws them 4 wide
pub fn render(f: &mut Frame, area: Rect, active: &[usize], colors: bool, scale: usize) {
    let bar = BLOCK.repeat(2 * scale);
    let col = 2 * scale + 1;
    let gap = " ".repeat(8 * scale);
    let color = |finger: usize| -> Color {
        match (active.contains(&finger), colors) {
            (true, true) => FINGERS[finger].1,
            (true, false) => ACCENT,
            (false, _) => KEYCAP,
        }
    };
    let cell = |finger: usize, row: u16| -> Span<'static> {
        let filled = row >= BAR_ROWS - TALL[finger];
        let text = if filled {
            bar.clone()
        } else {
            " ".repeat(2 * scale)
        };
        Span::styled(format!("{text} "), Style::new().fg(color(finger)))
    };
    // screen order: left fingers, thumb, gap, thumb, right fingers
    let left_order = [LEFT[0], LEFT[1], LEFT[2], LEFT[3], THUMB];
    let right_order = [THUMB, RIGHT[0], RIGHT[1], RIGHT[2], RIGHT[3]];
    let mut lines = Vec::new();
    for row in 0..BAR_ROWS {
        let mut spans: Vec<Span> = Vec::new();
        for (i, finger) in left_order.iter().enumerate() {
            if i == 4 {
                spans.push(Span::raw("  "));
            }
            spans.push(cell(*finger, row));
        }
        spans.push(Span::raw(gap.clone()));
        for (i, finger) in right_order.iter().enumerate() {
            if i == 1 {
                spans.push(Span::raw("  "));
            }
            spans.push(cell(*finger, row));
        }
        lines.push(Line::from(spans));
    }
    let tag = |f: &usize| format!("{:<col$}", TAGS[*f]);
    let left_tags: String =
        left_order[..4].iter().map(tag).collect::<String>() + "  " + &tag(&THUMB);
    let right_tags: String =
        tag(&THUMB) + "  " + &right_order[1..].iter().map(tag).collect::<String>();
    lines.push(Line::styled(format!("{left_tags}{gap}{right_tags}"), dim()));
    let hand_w = left_tags.len();
    lines.push(Line::styled(
        format!("{:^hand_w$}{gap}{:^hand_w$}", "left hand", "right hand"),
        dim(),
    ));
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}
