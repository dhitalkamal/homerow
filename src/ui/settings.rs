// the settings tab: a list of switches and a live preview of the keyboard
use super::{ACCENT, bold, centered, dim, hands, keyboard};
use crate::app::App;
use crate::settings::{ITEMS, Item};
use crate::store::Density;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// marker, padded label and the longest value, "auto (comfortable)"
const LIST_WIDTH: u16 = 46;
// the preview lights this key so you can see colors and hands at work
const SAMPLE: char = 'f';

// density is what auto resolved to, shown next to "auto" on the size row
pub fn render(f: &mut Frame, area: Rect, app: &App, density: Density) {
    let list_h = ITEMS.len() as u16;
    let opts = app.progress.settings;
    let hands_h = if opts.hands { hands::HEIGHT + 1 } else { 0 };
    let [_, list, _, caption, _, kbd, hand_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(list_h),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(keyboard::FULL_HEIGHT),
        Constraint::Length(hands_h),
        Constraint::Fill(1),
    ])
    .areas(area);
    let rows: Vec<Line> = ITEMS
        .iter()
        .enumerate()
        .map(|(i, item)| row(app, i, *item, density))
        .collect();
    f.render_widget(
        Paragraph::new(rows),
        centered(list, LIST_WIDTH, list.height),
    );
    let note = if opts.keyboard {
        "preview"
    } else {
        "preview   (the keyboard is off while you type)"
    };
    f.render_widget(
        Paragraph::new(Line::styled(note, dim())).alignment(Alignment::Center),
        caption,
    );
    keyboard::render(f, kbd, app, false, Some(SAMPLE));
    if opts.hands {
        let [_, h] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(hand_area);
        hands::render(f, h, &keyboard::fingers_for(SAMPLE), opts.finger_colors);
    }
}

fn row(app: &App, i: usize, item: Item, density: Density) -> Line<'static> {
    let selected = i == app.settings_row;
    let marker = if selected {
        Span::styled("> ", bold().fg(ACCENT))
    } else {
        Span::raw("  ")
    };
    let label_style = if selected {
        bold()
    } else {
        ratatui::style::Style::new()
    };
    let value = item.value(&app.progress);
    let shown = if item == Item::Size && app.progress.density == Density::Auto {
        format!("auto ({})", density.label())
    } else {
        value.to_string()
    };
    let value_style = match value {
        "on" => bold().fg(ACCENT),
        "off" => dim(),
        _ => bold(),
    };
    Line::from(vec![
        marker,
        Span::styled(format!("{:<24}", item.label()), label_style),
        Span::styled(shown, value_style),
    ])
}
