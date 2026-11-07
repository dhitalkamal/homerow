// the progress tab, one view at a time (all modes, baseline, weak keys, code):
// where you started, where you are now, the trend, weak keys and decks
use super::{ACCENT, bold, dim};
use crate::app::App;
use crate::cards::{Deck, known};
use crate::content::CARDS;
use crate::history::{self, BASE, CODE, KEYS, MISTAKES, View};
use crate::stats::{MIN_SAMPLES, avg_ms, weak_pairs, weakness};
use crate::store::{day_key, goal_streak};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, BorderType, Chart, Dataset, GraphType, Paragraph};

const TREND_ROUNDS: usize = 40;
const FULL: &str = "\u{2588}";
const EMPTY: &str = "\u{2591}";
// one line color per mode in the trend chart
const LINES: [(&str, &str, Color); 4] = [
    (BASE, "baseline", Color::Cyan),
    (KEYS, "weak keys", Color::Green),
    (CODE, "code", Color::Magenta),
    (MISTAKES, "mistakes", Color::Yellow),
];

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
    let view = app.progress_view;
    let deck_h = Deck::ALL.len() as u16 + 2;
    let [views, tiles, middle, decks] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(4),
        Constraint::Min(6),
        Constraint::Length(deck_h),
    ])
    .spacing(1)
    .areas(area);
    draw_view_tabs(f, views, view);
    match history::change(&app.progress, view) {
        None => {
            let what = if view == View::All {
                "baseline, weak keys or code"
            } else {
                view.label()
            };
            let msg = vec![
                Line::styled(format!("Finish a round in {what}"), bold()),
                Line::styled(
                    "and your speed, progress and weak keys show up here.",
                    dim(),
                ),
            ];
            let at = super::centered(middle, middle.width, 2);
            f.render_widget(Paragraph::new(msg).alignment(Alignment::Center), at);
        }
        Some(c) if app.show_history => {
            draw_tiles(f, tiles, app, view, c, now);
            super::history::render(f, middle, app, now);
        }
        Some(c) => {
            draw_tiles(f, tiles, app, view, c, now);
            let [trend, weak] =
                Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .spacing(2)
                    .areas(middle);
            draw_trend(f, trend, app, view);
            draw_weak(f, weak, app, view);
        }
    }
    draw_decks(f, decks, app);
}

fn draw_view_tabs(f: &mut Frame, area: Rect, view: View) {
    let mut spans = vec![Span::styled("< ", dim())];
    for (i, v) in View::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", dim()));
        }
        let style = if *v == view { bold().fg(ACCENT) } else { dim() };
        spans.push(Span::styled(v.label(), style));
    }
    spans.push(Span::styled(" >", dim()));
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

fn draw_tiles(f: &mut Frame, area: Rect, app: &App, view: View, c: history::Change, now: u64) {
    let rs = history::rounds(&app.progress, view);
    let recent: Vec<u32> = rs.iter().rev().take(10).map(|r| r.acc).collect();
    let acc = recent.iter().sum::<u32>() / recent.len().max(1) as u32;
    let today = app.progress.days.get(&day_key(now)).copied().unwrap_or(0) / 60_000;
    let goal = app.progress.settings.goal_min as u64;
    let sign = if c.delta() >= 0 { "+" } else { "" };
    let tiles = [
        (format!("{} wpm", c.now), "now (last 3 rounds)".to_string()),
        (
            format!("{} -> {}", c.first, c.now),
            format!("{sign}{} since first", c.delta()),
        ),
        (c.best.to_string(), "best wpm".to_string()),
        (format!("{acc}%"), format!("accuracy, {} rounds", c.rounds)),
        (
            match goal {
                0 => format!("{today} min today"),
                g => format!("{today}/{g} min today"),
            },
            format!(
                "{} day streak",
                goal_streak(&app.progress.days, now, goal as u32)
            ),
        ),
    ];
    let areas = Layout::horizontal([Constraint::Fill(1); 5])
        .spacing(1)
        .split(area);
    for ((value, label), a) in tiles.iter().zip(areas.iter()) {
        let body = vec![
            Line::styled(value.clone(), bold()),
            Line::styled(label.clone(), dim()),
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

// trend points per mode in the view, each counted from that mode's first round
// mode label, line color and (round, wpm) points
type Series = (&'static str, Color, Vec<(f64, f64)>);

fn series(app: &App, view: View) -> Vec<Series> {
    LINES
        .iter()
        .filter(|(tag, _, _)| view.modes().contains(tag))
        .map(|(tag, label, color)| {
            let wpms: Vec<u32> = app
                .progress
                .rounds
                .iter()
                .filter(|r| r.mode == *tag)
                .map(|r| r.wpm)
                .collect();
            let tail = &wpms[wpms.len().saturating_sub(TREND_ROUNDS)..];
            let pts: Vec<(f64, f64)> = tail
                .iter()
                .enumerate()
                .map(|(i, w)| (i as f64, *w as f64))
                .collect();
            (*label, *color, pts)
        })
        .filter(|(_, _, pts)| !pts.is_empty())
        .collect()
}

fn draw_trend(f: &mut Frame, area: Rect, app: &App, view: View) {
    let lines = series(app, view);
    let longest = lines.iter().map(|s| s.2.len()).max().unwrap_or(0);
    let max_wpm = lines
        .iter()
        .flat_map(|s| s.2.iter().map(|p| p.1 as u32))
        .max()
        .unwrap_or(0);
    // round the top up to the next 10, at least 60
    let top = ((max_wpm + 10).div_ceil(10) * 10).max(60) as f64;
    let data: Vec<Dataset> = lines
        .iter()
        .map(|(label, color, pts)| {
            Dataset::default()
                .name(*label)
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::new().fg(*color))
                .data(pts)
        })
        .collect();
    let title = format!("Speed trend, {}", view.label());
    let chart = Chart::new(data)
        .block(panel(&title))
        .x_axis(Axis::default().bounds([0.0, (longest.max(2) - 1) as f64]))
        .y_axis(Axis::default().bounds([0.0, top]).style(dim()).labels([
            "0".to_string(),
            format!("{}", top as u32 / 2),
            format!("{}", top as u32),
        ]));
    f.render_widget(chart, area);
}

fn draw_weak(f: &mut Frame, area: Rect, app: &App, view: View) {
    let stats = history::view_stats(&app.progress, view);
    let avg = avg_ms(&stats);
    let mut keys: Vec<(char, f64)> = stats
        .iter()
        .filter(|(_, s)| s.n >= MIN_SAMPLES)
        .map(|(c, s)| (*c, weakness(Some(s), avg)))
        .collect();
    keys.sort_by(|a, b| b.1.total_cmp(&a.1));
    // text views also list the slowest letter pairs on the last line
    let slow: Vec<String> = match view {
        View::All | View::Base | View::Keys => weak_pairs(&app.progress.pairs, 3),
        _ => Vec::new(),
    };
    let rows =
        (area.height.saturating_sub(2) as usize).saturating_sub(usize::from(!slow.is_empty()));
    let mut lines: Vec<Line> = if keys.is_empty() {
        vec![Line::styled(
            " type a few rounds to find your weak keys",
            dim(),
        )]
    } else {
        keys.iter()
            .take(rows)
            .map(|(c, w)| {
                let s = stats[c];
                let filled = (((w - 0.6) / 2.4).clamp(0.0, 1.0) * 10.0).round() as usize;
                let mut spans = vec![Span::styled(
                    format!(" {}  ", c.to_ascii_uppercase()),
                    bold(),
                )];
                spans.extend(bar(filled, 10, super::BAD));
                let miss = 100 * s.e / (s.n + s.e);
                let ms = s.ms / s.n as u64;
                spans.push(Span::styled(
                    format!("  {miss:>2}% miss  {ms:>3} ms"),
                    dim(),
                ));
                Line::from(spans)
            })
            .collect()
    };
    if !slow.is_empty() {
        let mut spans = vec![Span::styled(" slow pairs ", dim())];
        for pair in &slow {
            let s = app.progress.pairs[pair];
            spans.push(Span::styled(format!("  {pair}"), bold()));
            spans.push(Span::styled(
                format!(" {}ms", s.ms / s.n.max(1) as u64),
                dim(),
            ));
        }
        lines.push(Line::from(spans));
    }
    let title = if view == View::All {
        "Weak keys, text".to_string()
    } else {
        format!("Weak keys, {}", view.label())
    };
    f.render_widget(Paragraph::new(lines).block(panel(&title)), area);
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
