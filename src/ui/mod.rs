// drawing. reads App, never changes it.
// colors come from the terminal's own ansi palette, so they follow your theme.
pub mod bigtext;
pub mod card;
pub mod dashboard;
pub mod focus;
pub mod hands;
pub mod history;
pub mod keyboard;
pub mod layout;
pub mod prompt;
pub mod settings;

use crate::app::{App, Mode};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// the one accent: cursor, active tab, live numbers
pub const ACCENT: Color = Color::Rgb(255, 196, 92);
pub const BAD: Color = Color::Red;
pub const GOOD: Color = Color::Green;
pub const KEYCAP: Color = Color::DarkGray;

pub fn dim() -> Style {
    Style::new().add_modifier(Modifier::DIM)
}
pub fn bold() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

// zoom collects kitty text sizing runs; main.rs writes them after the frame
pub fn draw(f: &mut Frame, app: &App, now: u64, zoom: &mut Vec<crate::zoom::Run>) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area().inner(ratatui::layout::Margin::new(2, 1)));
    draw_header(f, header, app);
    // auto size is decided once per frame from the full terminal size
    let full = f.area();
    let density = app.progress.density.resolve(full.width, full.height);
    match app.mode {
        Mode::Cards => card::render(f, body, app),
        Mode::Progress => dashboard::render(f, body, app, now),
        Mode::Settings => settings::render(f, body, app, density),
        _ => focus::render(f, body, app, now, density, zoom),
    }
    draw_footer(f, footer, app, now);
}

// a centered box of at most w x h inside area
pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let mut tabs = Vec::new();
    for (i, m) in Mode::ALL.iter().enumerate() {
        if i > 0 {
            tabs.push(Span::styled("  ", dim()));
        }
        let style = if *m == app.mode {
            bold().fg(ACCENT)
        } else {
            dim()
        };
        tabs.push(Span::styled(m.label(), style));
    }
    let [left, mid] = Layout::horizontal([Constraint::Length(12), Constraint::Min(0)]).areas(area);
    f.render_widget(
        Paragraph::new(Span::styled(crate::brand::NAME, bold())),
        left,
    );
    f.render_widget(
        Paragraph::new(Line::from(tabs)).alignment(Alignment::Center),
        mid,
    );
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App, now: u64) {
    // (key, what it does): key in normal weight, action dimmed
    let hints: &[(&str, &str)] = match app.mode {
        Mode::Cards => &[
            ("tab", "mode"),
            ("esc", "quit"),
            ("space", "reveal"),
            ("[ ]", "deck"),
        ],
        Mode::Progress => &[
            ("left/right", "view"),
            ("return", "history"),
            ("tab", "mode"),
            ("esc", "quit"),
        ],
        Mode::Settings => &[
            ("up/down", "move"),
            ("space", "change"),
            ("left/right", "set"),
            ("tab", "mode"),
            ("esc", "quit"),
        ],
        _ if app.session.is_done() => &[
            ("return", "next round"),
            ("c", "copy share card"),
            ("tab", "mode"),
            ("esc", "quit"),
        ],
        _ => &[("tab", "mode"), ("esc", "quit"), ("ctrl + -", "size")],
    };
    // an error beats a size toast, a toast beats the key hints
    let line = match (&app.notice, app.toast(now)) {
        (Some(n), _) => Line::styled(n.as_str(), Style::new().fg(BAD)),
        (None, Some(t)) => Line::styled(t.to_string(), bold().fg(ACCENT)),
        (None, None) => Line::from(
            hints
                .iter()
                .enumerate()
                .flat_map(|(i, (k, what))| {
                    let gap = if i == 0 { "" } else { "    " };
                    [Span::raw(format!("{gap}{k} ")), Span::styled(*what, dim())]
                })
                .collect::<Vec<_>>(),
        ),
    };
    f.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_history;
