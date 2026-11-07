// the progress tab: headline numbers, speed trend, weak keys, deck progress
use super::{bold, dim};
use crate::app::App;
use crate::cards::{Deck, known};
use crate::content::CARDS;
use crate::stats::{MIN_SAMPLES, avg_ms, weakness};
use crate::store::{day_key, streak};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, BorderType, Chart, Dataset, GraphType, Paragraph};

const TREND_ROUNDS: usize = 40;
const FULL: &str = "\u{2588}";
const EMPTY: &str = "\u{2591}";
const CHART: Color = Color::Cyan;

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(dim())
        .title(Span::styled(format!(" {title} "), bold()))
}

// a bar of `width` cells, `filled` of them solid
fn bar(filled: usize, width: usize, color: Color) -> [Span<'static>; 2] {
    let filled = filled.min(width);
    [
        Span::styled(FULL.repeat(filled), Style::new().fg(color)),
        Span::styled(EMPTY.repeat(width - filled), dim()),
    ]
}

pub fn render(f: &mut Frame, area: Rect, app: &App, now: u64) {
    let deck_h = Deck::ALL.len() as u16 + 2;
    let [tiles, middle, decks] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(6),
        Constraint::Length(deck_h),
    ])
    .spacing(1)
    .areas(area);
    let wpms: Vec<u32> = app
        .progress
        .rounds
        .iter()
        .filter(|r| r.mode != "cards")
        .map(|r| r.wpm)
        .collect();
    if wpms.is_empty() {
        let msg = vec![
            Line::styled("Finish a round in baseline, weak keys or code", bold()),
            Line::styled("and your speed, streak and weak keys show up here.", dim()),
        ];
        f.render_widget(
            Paragraph::new(msg).alignment(Alignment::Center),
            super::centered(middle, middle.width, 2),
        );
    } else {
        draw_tiles(f, tiles, app, &wpms, now);
        let [trend, weak] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .spacing(2)
                .areas(middle);
        draw_trend(f, trend, &wpms);
        draw_weak(f, weak, app);
    }
    draw_decks(f, decks, app);
}

fn draw_tiles(f: &mut Frame, area: Rect, app: &App, wpms: &[u32], now: u64) {
    let recent: Vec<u32> = app
        .progress
        .rounds
        .iter()
        .rev()
        .filter(|r| r.mode != "cards")
        .take(10)
        .map(|r| r.acc)
        .collect();
    let acc = recent.iter().sum::<u32>() / recent.len().max(1) as u32;
    let today = app.progress.days.get(&day_key(now)).copied().unwrap_or(0) / 60_000;
    let tiles = [
        (wpms.last().copied().unwrap_or(0).to_string(), "latest wpm"),
        (
            wpms.iter().max().copied().unwrap_or(0).to_string(),
            "best wpm",
        ),
        (format!("{acc}%"), "accuracy, last 10"),
        (streak(&app.progress.days, now).to_string(), "day streak"),
        (format!("{today}m"), "practiced today"),
    ];
    let areas = Layout::horizontal([Constraint::Fill(1); 5])
        .spacing(1)
        .split(area);
    for ((value, label), a) in tiles.iter().zip(areas.iter()) {
        let body = vec![
            Line::styled(value.clone(), bold()),
            Line::styled(*label, dim()),
        ];
        let tile = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(dim());
        f.render_widget(
            Paragraph::new(body)
                .alignment(Alignment::Center)
                .block(tile),
            *a,
        );
    }
}

fn draw_trend(f: &mut Frame, area: Rect, wpms: &[u32]) {
    let tail = &wpms[wpms.len().saturating_sub(TREND_ROUNDS)..];
    let points: Vec<(f64, f64)> = tail
        .iter()
        .enumerate()
        .map(|(i, w)| (i as f64, *w as f64))
        .collect();
    // round the top up to the next 10, at least 60
    let top = ((tail.iter().max().copied().unwrap_or(0) + 10).div_ceil(10) * 10).max(60) as f64;
    let data = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::new().fg(CHART))
        .data(&points);
    let title = format!("Speed trend, last {} rounds", tail.len());
    let chart = Chart::new(vec![data])
        .block(panel(&title))
        .x_axis(Axis::default().bounds([0.0, (tail.len().max(2) - 1) as f64]))
        .y_axis(Axis::default().bounds([0.0, top]).style(dim()).labels([
            "0".to_string(),
            format!("{}", top as u32 / 2),
            format!("{}", top as u32),
        ]));
    f.render_widget(chart, area);
}

fn draw_weak(f: &mut Frame, area: Rect, app: &App) {
    let avg = avg_ms(&app.progress.stats);
    let mut keys: Vec<(char, f64)> = app
        .progress
        .stats
        .iter()
        .filter(|(_, s)| s.n >= MIN_SAMPLES)
        .map(|(c, s)| (*c, weakness(Some(s), avg)))
        .collect();
    keys.sort_by(|a, b| b.1.total_cmp(&a.1));
    let rows = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = keys
        .iter()
        .take(rows)
        .map(|(c, w)| {
            let s = app.progress.stats[c];
            let filled = (((w - 0.6) / 2.4).clamp(0.0, 1.0) * 10.0).round() as usize;
            let mut spans = vec![Span::styled(
                format!(" {}  ", c.to_ascii_uppercase()),
                bold(),
            )];
            spans.extend(bar(filled, 10, super::BAD));
            spans.push(Span::styled(
                format!(
                    "  {:>2}% miss  {:>3} ms",
                    100 * s.e / (s.n + s.e),
                    s.ms / s.n as u64
                ),
                dim(),
            ));
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(panel("Weak keys")), area);
}

fn draw_decks(f: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = Deck::ALL
        .iter()
        .map(|&d| {
            let (k, n) = known(CARDS, &app.progress.boxes, d);
            let mut spans = vec![Span::raw(format!(" {:<16}", d.label()))];
            spans.extend(bar((k * 24).checked_div(n).unwrap_or(0), 24, super::GOOD));
            spans.push(Span::styled(format!("  {k}/{n} learned"), dim()));
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(panel("Decks")), area);
}
