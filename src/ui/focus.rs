// the typing screen: only the text, one live stats line and the keyboard.
// after a round it becomes the results screen with the speed in big digits.
use super::{bigtext, bold, centered, dim, keyboard, prompt};
use crate::app::{App, Mode};
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// below this body height the keyboard is dropped so the text keeps its room
const KEYBOARD_MIN_HEIGHT: u16 = 26;
const TEXT_WIDTH: u16 = 70;

pub fn render(f: &mut Frame, area: Rect, app: &App, now: u64) {
    if app.session.is_done() {
        return results(f, area, app);
    }
    let kbd_h = if area.height >= KEYBOARD_MIN_HEIGHT {
        keyboard::HEIGHT
    } else {
        0
    };
    let [_, hint, _, text, stats, _, kbd, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(if kbd_h > 0 { 2 } else { 0 }),
        Constraint::Length(kbd_h),
        Constraint::Fill(1),
    ])
    .areas(area);
    let center = |r: Rect| centered(r, TEXT_WIDTH, r.height);
    f.render_widget(
        Paragraph::new(hint_line(app)).alignment(Alignment::Center),
        hint,
    );
    prompt::render(f, center(text), app);
    f.render_widget(
        Paragraph::new(stats_line(app, now)).alignment(Alignment::Center),
        stats,
    );
    if kbd_h > 0 {
        keyboard::render(f, kbd, app);
    }
}

fn hint_line(app: &App) -> Line<'static> {
    match app.mode {
        Mode::Keys => {
            let mut spans = vec![Span::styled("focus  ", dim())];
            for c in &app.focus {
                spans.push(Span::styled(
                    format!(" {} ", c.to_ascii_uppercase()),
                    bold(),
                ));
            }
            Line::from(spans)
        }
        Mode::Code => Line::styled("real code, indentation is typed for you", dim()),
        _ => Line::styled("60 second test, the clock starts on your first key", dim()),
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
    Line::from(vec![
        Span::styled(s.wpm(now).to_string(), bold()),
        Span::styled(" wpm      ", dim()),
        Span::styled(format!("{}%", s.accuracy()), bold()),
        Span::styled(" accuracy      ", dim()),
        Span::styled(clock, dim()),
    ])
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
