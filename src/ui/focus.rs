// the typing screen: only the text, one live stats line and the keyboard.
// after a round it becomes the results screen with the speed in big digits.
use super::layout::{BIG_ROW, Kbd, plan};
use super::{bigtext, bold, centered, dim, hands, keyboard, prompt};
use crate::app::{App, Mode};
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use crate::store::Density;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// density is already resolved (never auto) by the caller
pub fn render(
    f: &mut Frame,
    area: Rect,
    app: &App,
    now: u64,
    density: Density,
    zoom: &mut Vec<crate::zoom::Run>,
) {
    if app.session.is_done() {
        return results(f, area, app);
    }
    let opts = app.progress.settings;
    let p = plan(
        area.width,
        area.height,
        density,
        opts.keyboard,
        opts.hands,
        opts.big_text,
        if app.zoom_ok { opts.zoom as u16 } else { 1 },
    );
    let kbd_h = match p.kbd {
        Kbd::Large => keyboard::LARGE_HEIGHT,
        Kbd::Full => keyboard::FULL_HEIGHT,
        Kbd::Compact => keyboard::COMPACT_HEIGHT,
        Kbd::Hidden => 0,
    };
    let text_h = p.rows
        * if p.zoom > 1 {
            p.zoom + 1
        } else if p.big {
            BIG_ROW
        } else if p.spaced {
            2
        } else {
            1
        };
    let hands_h = if p.hands { hands::HEIGHT } else { 0 };
    // a little above center reads as centered (more room below than above)
    let [_, text, _, stats, _, kbd, _, hand_area, _] = Layout::vertical([
        Constraint::Fill(4),
        Constraint::Length(text_h),
        Constraint::Length(if p.spaced { 0 } else { 1 }),
        Constraint::Length(1),
        Constraint::Length(if kbd_h > 0 { 2 } else { 0 }),
        Constraint::Length(kbd_h),
        Constraint::Length(if p.hands { 1 } else { 0 }),
        Constraint::Length(hands_h),
        Constraint::Fill(5),
    ])
    .areas(area);
    let text_area = centered(text, p.text_width, text.height);
    if p.zoom > 1 {
        zoom.extend(prompt::zoom_runs(text_area, app, p.rows, p.zoom));
    } else if p.big {
        prompt::render_big(f, text_area, app, p.rows);
    } else {
        prompt::render(f, text_area, app, p.spaced);
    }
    f.render_widget(
        Paragraph::new(stats_line(app, now)).alignment(Alignment::Center),
        stats,
    );
    if kbd_h > 0 {
        let next = app.session.next_char();
        let scale = if p.kbd == Kbd::Large {
            keyboard::SCALE_LARGE
        } else {
            1
        };
        keyboard::render(f, kbd, app, p.kbd == Kbd::Compact, scale, next);
        if p.hands {
            let lit = next.map(keyboard::fingers_for).unwrap_or_default();
            hands::render(f, hand_area, &lit, opts.finger_colors, scale);
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
