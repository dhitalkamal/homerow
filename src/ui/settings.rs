// the settings tab: a list of switches and a live preview of the keyboard
use super::{ACCENT, bold, centered, dim, hands, keyboard};
use crate::app::App;
use crate::settings::{ITEMS, Item};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

const LIST_WIDTH: u16 = 40;
// the preview lights this key so you can see colors and hands at work
const SAMPLE: char = 'f';

pub fn render(f: &mut Frame, area: Rect, app: &App) {
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
        .map(|(i, item)| row(app, i, *item))
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

fn row(app: &App, i: usize, item: Item) -> Line<'static> {
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
    let value_style = match value {
        "on" => bold().fg(ACCENT),
        "off" => dim(),
        _ => bold(),
    };
    Line::from(vec![
        marker,
        Span::styled(format!("{:<24}", item.label()), label_style),
        Span::styled(value, value_style),
    ])
}
