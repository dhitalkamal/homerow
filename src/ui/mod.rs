// drawing. reads App, never changes it.
pub mod card;
pub mod keyboard;
pub mod prompt;

use crate::app::{App, Mode};
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use crate::store::day_key;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Sparkline};

pub const ACCENT: Color = Color::Rgb(226, 87, 43);
pub const BAD: Color = Color::Rgb(255, 100, 100);
pub const GOOD: Color = Color::Rgb(76, 196, 136);
pub const DIM: Color = Color::Rgb(128, 134, 148);

pub fn draw(f: &mut Frame, app: &App, now: u64) {
    let typing = app.mode != Mode::Cards;
    let [header, stats, main, kbd, progress, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(6),
        Constraint::Length(if typing { 7 } else { 0 }),
        Constraint::Length(7),
        Constraint::Length(1),
    ])
    .areas(f.area().inner(ratatui::layout::Margin::new(2, 1)));
    draw_header(f, header, app);
    draw_stats(f, stats, app, now);
    if typing {
        draw_typing(f, main, app);
        keyboard::render(f, kbd, app);
    } else {
        card::render(f, main, app);
    }
    draw_progress(f, progress, app);
    draw_footer(f, footer, app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled("Home Row", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled(".   ", Style::new().fg(ACCENT)),
    ];
    for m in Mode::ALL {
        let style = if m == app.mode {
            Style::new()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(DIM)
        };
        spans.push(Span::styled(format!(" {} ", m.label()), style));
        spans.push(Span::raw(" "));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_stats(f: &mut Frame, area: Rect, app: &App, now: u64) {
    let s = &app.session;
    let live = app.mode != Mode::Cards && s.started() && !s.is_done();
    let last = app.last_round();
    let (wpm, acc) = if live {
        (s.wpm(now), s.accuracy())
    } else {
        last.map_or((0, 0), |r| (r.wpm, r.acc))
    };
    let best = app
        .progress
        .rounds
        .iter()
        .filter(|r| r.mode != "cards")
        .map(|r| r.wpm)
        .max()
        .unwrap_or(0);
    let today = app.progress.days.get(&day_key(now)).copied().unwrap_or(0) / 60_000;
    let big = Style::new().add_modifier(Modifier::BOLD);
    let label = Style::new().fg(DIM);
    let mut spans = vec![
        Span::styled(format!("{wpm}"), big),
        Span::styled(" wpm     ", label),
        Span::styled(
            if acc == 0 {
                "-".into()
            } else {
                format!("{acc}%")
            },
            big,
        ),
        Span::styled(" accuracy     ", label),
        Span::styled(format!("{best}"), big),
        Span::styled(" best wpm     ", label),
        Span::styled(format!("{today}m"), big),
        Span::styled(" practiced today", label),
    ];
    if app.mode == Mode::Base && live {
        spans.push(Span::styled(
            format!("     {}s left", 60u64.saturating_sub(s.elapsed(now) / 1000)),
            Style::new().fg(ACCENT),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_typing(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.mode {
        Mode::Keys => format!(
            " Focus letters: {} ",
            app.focus
                .iter()
                .map(|c| c.to_ascii_uppercase().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Mode::Code => " Real code. Return for new lines, indentation is typed for you ".into(),
        _ => " Steady pace. Accuracy first, speed follows ".into(),
    };
    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(DIM))
        .padding(Padding::new(2, 2, 1, 0));
    let inner = block.inner(area);
    f.render_widget(block, area);
    match (app.session.is_done(), app.last_round()) {
        (true, Some(r)) => {
            let msg = if r.acc < 95 {
                "Accuracy under 95%. Slow down a little, speed comes from clean reps."
            } else if r.wpm < 40 {
                "Good start. Keep your eyes off the keyboard, even when it feels slower."
            } else {
                "Solid. Try Weak keys next to clean up your slowest letters."
            };
            let lines = vec![
                Line::styled(
                    format!("{} wpm, {}% accuracy", r.wpm, r.acc),
                    Style::new().add_modifier(Modifier::BOLD),
                ),
                Line::raw(""),
                Line::raw(msg),
                Line::raw(""),
                Line::styled("Press Return for the next round", Style::new().fg(ACCENT)),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        _ => prompt::render(f, inner, app),
    }
}

fn draw_progress(f: &mut Frame, area: Rect, app: &App) {
    let [trend, weak] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
            .spacing(2)
            .areas(area);
    let dim = Style::new().fg(DIM);
    let rounds: Vec<u64> = app
        .progress
        .rounds
        .iter()
        .filter(|r| r.mode != "cards")
        .map(|r| r.wpm as u64)
        .collect();
    let tail = &rounds[rounds
        .len()
        .saturating_sub(trend.width.saturating_sub(2) as usize)..];
    let title = if tail.is_empty() {
        " Speed trend: finish a round to start it ".into()
    } else {
        format!(" Speed trend, last {} rounds ", tail.len())
    };
    let spark = Sparkline::default()
        .data(tail)
        .max(tail.iter().copied().max().unwrap_or(0).max(60))
        .style(Style::new().fg(ACCENT));
    f.render_widget(
        spark.block(
            Block::new()
                .title(title)
                .borders(Borders::TOP)
                .border_style(dim),
        ),
        trend,
    );

    let avg = avg_ms(&app.progress.stats);
    let mut tried: Vec<(char, f64)> = app
        .progress
        .stats
        .iter()
        .filter(|(_, s)| s.n >= MIN_SAMPLES)
        .map(|(c, s)| (*c, weakness(Some(s), avg)))
        .collect();
    tried.sort_by(|a, b| b.1.total_cmp(&a.1));
    let lines: Vec<Line> = if tried.is_empty() {
        vec![Line::styled(
            "Type a round and your weakest letters land here.",
            dim,
        )]
    } else {
        tried
            .iter()
            .take(5)
            .map(|(c, _)| {
                let s = app.progress.stats[c];
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", c.to_ascii_uppercase()),
                        Style::new()
                            .bg(BAD)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(
                            "  {:>3}% missed   {:>4} ms avg",
                            100 * s.e / (s.n + s.e),
                            s.ms / s.n as u64
                        ),
                        dim,
                    ),
                ])
            })
            .collect()
    };
    f.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .title(" Slowest and most missed keys ")
                .borders(Borders::TOP)
                .border_style(dim),
        ),
        weak,
    );
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let line = match (&app.notice, app.mode) {
        (Some(n), _) => Line::styled(n.as_str(), Style::new().fg(BAD)),
        (None, Mode::Cards) => Line::styled(
            "Tab mode   Esc quit   Space reveal   [ ] change deck",
            Style::new().fg(DIM),
        ),
        (None, _) => Line::styled(
            "Tab mode   Esc quit   ctrl+F finger colors   ctrl+K heatmap   ctrl+L hide labels",
            Style::new().fg(DIM),
        ),
    };
    f.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Input, Mode};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn screen(app: &App, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| draw(f, app, 0)).unwrap();
        let buf = t.backend().buffer();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    fn app() -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (App::new(dir.path().join("p.json"), 5), dir)
    }

    #[test]
    fn typing_screen_shows_tabs_prompt_and_keyboard() {
        let (a, _d) = app();
        let s = screen(&a, 100, 40);
        for want in [
            "Baseline 60s",
            "Weak keys",
            "Shortcuts",
            "wpm",
            "L index",
            "Tab",
        ] {
            assert!(s.contains(want), "missing {want:?} in\n{s}");
        }
        let first: String = a.session.text.iter().take_while(|c| **c != ' ').collect();
        assert!(s.contains(&first));
    }

    #[test]
    fn shortcut_screen_shows_the_card_and_decks() {
        let (mut a, _d) = app();
        a.handle(Input::BackTab, 0);
        assert_eq!(a.mode, Mode::Cards);
        let s = screen(&a, 100, 40);
        let card = crate::content::CARDS[a.card];
        assert!(s.contains(card.action), "missing action in\n{s}");
        assert!(s.contains("Terminal (zsh)"));
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        let (a, _d) = app();
        screen(&a, 20, 6);
    }
}
