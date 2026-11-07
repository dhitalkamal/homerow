// the round list on the progress tab: every round in the current view,
// newest first, scrolled with up/down
use super::{bold, dim};
use crate::app::App;
use crate::history::{ago, history_rows};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};

pub fn render(f: &mut Frame, area: Rect, app: &App, now: u64) {
    let rows = history_rows(&app.progress, app.progress_view);
    let visible = area.height.saturating_sub(3) as usize;
    let mut lines = vec![Line::styled(
        format!(
            "  {:<12}{:<12}{:>8}{:>10}",
            "when", "mode", "wpm", "accuracy"
        ),
        dim(),
    )];
    for r in rows.iter().skip(app.history_scroll).take(visible) {
        lines.push(Line::from(vec![
            Span::raw(format!("  {:<12}", ago(now, r.at_ms))),
            Span::styled(format!("{:<12}", label(&r.mode)), dim()),
            Span::styled(format!("{:>8}", r.wpm), bold()),
            Span::raw(format!("{:>9}%", r.acc)),
        ]));
    }
    let title = format!(
        " Rounds, {} of {}  (up/down scroll, return closes) ",
        (app.history_scroll + 1).min(rows.len()),
        rows.len()
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(dim())
        .title(Span::styled(title, bold()));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

// round tags as people read them
fn label(tag: &str) -> &str {
    match tag {
        "base" => "baseline",
        "keys" => "weak keys",
        other => other,
    }
}
