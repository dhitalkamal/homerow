// the typing screen: only the text, one live stats line and the keyboard.
// after a round it becomes the results screen with the speed in big digits.
use super::{bigtext, bold, centered, dim, hands, keyboard, prompt};
use crate::app::{App, Mode};
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use crate::store::Density;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// the keyboard needs this many columns, narrower windows drop it
const KEYBOARD_MIN_WIDTH: u16 = 62;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kbd {
    Full,
    Compact,
    Hidden,
}

// how the typing screen fits a body area of this size at this density
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub rows: u16,
    // a blank line after each text row
    pub spaced: bool,
    pub text_width: u16,
    pub kbd: Kbd,
    // finger bars under the keyboard
    pub hands: bool,
}

// keyboard and hands come from settings; they only show when there is room
pub fn plan(width: u16, height: u16, density: Density, keyboard: bool, hands: bool) -> Plan {
    let (mut rows, spaced, w) = match density {
        Density::Compact => (3, false, 60),
        Density::Comfortable | Density::Auto => (3, true, 70),
        Density::Spacious => (4, true, 84),
    };
    let per = if spaced { 2 } else { 1 };
    // the stats line, plus a blank above it when rows are not spaced
    let fixed = if spaced { 1 } else { 2 };
    while rows > 1 && rows * per + fixed > height {
        rows -= 1;
    }
    // text block, stats, and a 2 line gap above the keyboard
    let needed = rows * per + fixed + 2;
    let kbd = if !keyboard || density == Density::Compact || width < KEYBOARD_MIN_WIDTH {
        Kbd::Hidden
    } else if height >= needed + keyboard::FULL_HEIGHT + 2 {
        Kbd::Full
    } else if height >= needed + keyboard::COMPACT_HEIGHT + 2 {
        Kbd::Compact
    } else {
        Kbd::Hidden
    };
    let kbd_h = match kbd {
        Kbd::Full => keyboard::FULL_HEIGHT,
        Kbd::Compact => keyboard::COMPACT_HEIGHT,
        Kbd::Hidden => 0,
    };
    // hands go first when space runs out, the keyboard is more useful
    let hands = hands && kbd != Kbd::Hidden && height >= needed + kbd_h + hands::HEIGHT + 1 + 2;
    Plan {
        rows,
        spaced,
        text_width: w.min(width),
        kbd,
        hands,
    }
}

// density is already resolved (never auto) by the caller
pub fn render(f: &mut Frame, area: Rect, app: &App, now: u64, density: Density) {
    if app.session.is_done() {
        return results(f, area, app);
    }
    let opts = app.progress.settings;
    let p = plan(area.width, area.height, density, opts.keyboard, opts.hands);
    let kbd_h = match p.kbd {
        Kbd::Full => keyboard::FULL_HEIGHT,
        Kbd::Compact => keyboard::COMPACT_HEIGHT,
        Kbd::Hidden => 0,
    };
    let text_h = p.rows * if p.spaced { 2 } else { 1 };
    let hands_h = if p.hands { hands::HEIGHT } else { 0 };
    let [_, text, _, stats, _, kbd, _, hand_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(text_h),
        Constraint::Length(if p.spaced { 0 } else { 1 }),
        Constraint::Length(1),
        Constraint::Length(if kbd_h > 0 { 2 } else { 0 }),
        Constraint::Length(kbd_h),
        Constraint::Length(if p.hands { 1 } else { 0 }),
        Constraint::Length(hands_h),
        Constraint::Fill(1),
    ])
    .areas(area);
    prompt::render(f, centered(text, p.text_width, text.height), app, p.spaced);
    f.render_widget(
        Paragraph::new(stats_line(app, now)).alignment(Alignment::Center),
        stats,
    );
    if kbd_h > 0 {
        let next = app.session.next_char();
        keyboard::render(f, kbd, app, p.kbd == Kbd::Compact, next);
        if p.hands {
            let lit = next.map(keyboard::fingers_for).unwrap_or_default();
            hands::render(f, hand_area, &lit, opts.finger_colors);
        }
    }
}

// shown from the start (dim at zero) so the line never jumps when you begin
fn stats_line(app: &App, now: u64) -> Line<'static> {
    let s = &app.session;
    let secs = s.elapsed(now) / 1000;
    let clock = if app.mode == Mode::Base {
        format!("{}s left", 60u64.saturating_sub(secs))
    } else {
        format!("{secs}s")
    };
    let mut line = Line::from(vec![
        Span::styled(s.wpm(now).to_string(), bold().fg(super::ACCENT)),
        Span::styled(" wpm      ", dim()),
        Span::styled(format!("{}%", s.accuracy()), bold().fg(super::ACCENT)),
        Span::styled(" accuracy      ", dim()),
        Span::styled(clock, dim()),
    ]);
    // weak keys: which letters this round leans on
    if app.mode == Mode::Keys {
        line.push_span(Span::styled("      focus ", dim()));
        for c in &app.focus {
            line.push_span(Span::styled(format!(" {}", c.to_ascii_uppercase()), bold()));
        }
    }
    line
}

fn results(f: &mut Frame, area: Rect, app: &App) {
    let Some(r) = app.last_round() else { return };
    let typing: Vec<_> = app
        .progress
        .rounds
        .iter()
        .filter(|x| x.mode != "cards")
        .collect();
    let best = typing.iter().map(|x| x.wpm).max().unwrap_or(0);
    let prev = typing.iter().rev().skip(1).find(|x| x.mode == r.mode);
    let mut lines: Vec<Line> = bigtext::digits(r.wpm).into_iter().map(Line::raw).collect();
    lines.push(Line::styled("words per minute", dim()));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(format!("{}%", r.acc), bold()),
        Span::styled(" accuracy      best ", dim()),
        Span::styled(best.to_string(), bold()),
    ]));
    let (delta, style) = match prev {
        _ if r.wpm >= best && typing.len() > 1 => {
            ("new best".to_string(), Style::new().fg(super::GOOD))
        }
        Some(p) if r.wpm >= p.wpm => (
            format!("+{} since last time", r.wpm - p.wpm),
            Style::new().fg(super::GOOD),
        ),
        Some(p) => (
            format!("-{} since last time", p.wpm - r.wpm),
            Style::new().fg(super::BAD),
        ),
        None => ("first round in this mode".to_string(), dim()),
    };
    lines.push(Line::styled(delta, style));
    lines.push(Line::raw(""));
    let slow = slowest(app, 3);
    if !slow.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("slowest keys   ", dim()),
            Span::styled(slow, bold()),
        ]));
    }
    let advice = if r.acc < 95 {
        "under 95% accuracy, slow down a little, clean reps build speed"
    } else if r.wpm < 40 {
        "keep your eyes off the keyboard, even when it feels slower"
    } else {
        "solid, try weak keys next to clean up your slowest letters"
    };
    lines.push(Line::styled(advice, dim()));
    lines.push(Line::raw(""));
    lines.push(Line::styled("press return for the next round", bold()));
    let h = lines.len() as u16;
    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        centered(area, area.width, h),
    );
}

fn slowest(app: &App, k: usize) -> String {
    let avg = avg_ms(&app.progress.stats);
    let mut v: Vec<(char, f64)> = app
        .progress
        .stats
        .iter()
        .filter(|(_, s)| s.n >= MIN_SAMPLES)
        .map(|(c, s)| (*c, weakness(Some(s), avg)))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.iter()
        .take(k)
        .map(|(c, _)| c.to_ascii_uppercase().to_string())
        .collect::<Vec<_>>()
        .join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use Density::*;

    fn on(w: u16, h: u16, d: Density) -> Plan {
        plan(w, h, d, true, true)
    }

    #[test]
    fn keyboard_off_means_text_only() {
        let p = plan(120, 50, Comfortable, false, true);
        assert_eq!((p.kbd, p.hands), (Kbd::Hidden, false));
    }

    #[test]
    fn comfortable_steps_down_hands_then_keyboard_as_space_shrinks() {
        let big = on(100, 40, Comfortable);
        assert_eq!((big.kbd, big.hands), (Kbd::Full, true));
        let mid = on(100, 26, Comfortable);
        assert_eq!((mid.kbd, mid.hands), (Kbd::Full, false));
        assert_eq!(on(100, 20, Comfortable).kbd, Kbd::Compact);
        assert_eq!(on(100, 13, Comfortable).kbd, Kbd::Hidden);
        assert_eq!((big.rows, big.spaced, big.text_width), (3, true, 70));
    }

    #[test]
    fn hands_switch_off_keeps_the_keyboard() {
        let p = plan(100, 40, Comfortable, true, false);
        assert_eq!((p.kbd, p.hands), (Kbd::Full, false));
    }

    #[test]
    fn narrow_window_hides_the_keyboard_and_shrinks_the_text() {
        let p = on(50, 40, Comfortable);
        assert_eq!((p.kbd, p.hands, p.text_width), (Kbd::Hidden, false, 50));
    }

    #[test]
    fn compact_density_is_text_only_and_tight() {
        let p = on(100, 40, Compact);
        assert_eq!(
            p,
            Plan {
                rows: 3,
                spaced: false,
                text_width: 60,
                kbd: Kbd::Hidden,
                hands: false
            }
        );
    }

    #[test]
    fn spacious_density_is_wider_with_more_rows() {
        let p = on(120, 45, Spacious);
        assert_eq!(
            (p.rows, p.spaced, p.text_width, p.kbd),
            (4, true, 84, Kbd::Full)
        );
        assert_eq!(on(70, 40, Spacious).text_width, 70);
    }

    #[test]
    fn tiny_area_still_fits_at_least_one_row() {
        for d in [Compact, Comfortable, Spacious] {
            let p = on(20, 4, d);
            let lines = p.rows * if p.spaced { 2 } else { 1 } + 1;
            assert!(p.rows >= 1 && lines <= 4, "{d:?} {p:?}");
            assert_eq!(p.kbd, Kbd::Hidden);
        }
    }
}
