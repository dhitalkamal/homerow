// the shortcut card, centered: what to do, the keys as keycaps, and how it went
use super::{bold, dim};
use crate::app::{App, CardState};
use crate::cards::{Combo, Key, known};
use crate::content::CARDS;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

fn glyph(cp: u32, name: &str) -> String {
    let g = char::from_u32(cp).map(String::from).unwrap_or_default();
    if name.is_empty() {
        g
    } else {
        format!("{g} {name}")
    }
}

fn token(t: &str) -> String {
    match t {
        "cmd" => glyph(0x2318, "cmd"),
        "opt" => glyph(0x2325, "opt"),
        "ctrl" => glyph(0x2303, "ctrl"),
        "shift" => glyph(0x21E7, "shift"),
        "Left" => glyph(0x2190, ""),
        "Up" => glyph(0x2191, ""),
        "Right" => glyph(0x2192, ""),
        "Down" => glyph(0x2193, ""),
        "Back" => glyph(0x232B, "delete"),
        "Esc" => "esc".into(),
        "Enter" => "return".into(),
        "Space" => "space".into(),
        "Tab" => "tab".into(),
        t => t.to_uppercase(),
    }
}

pub fn combo_labels(s: &str) -> Vec<String> {
    s.split(['+', ' ']).map(token).collect()
}

// what you actually pressed, in mac modifier order
pub fn pressed_labels(c: &Combo) -> Vec<String> {
    let mut out = Vec::new();
    for (on, t) in [
        (c.mods.ctrl, "ctrl"),
        (c.mods.opt, "opt"),
        (c.mods.shift, "shift"),
        (c.mods.cmd, "cmd"),
    ] {
        if on {
            out.push(token(t));
        }
    }
    out.push(match c.key {
        Key::Char(ch) => ch.to_uppercase().to_string(),
        Key::F(n) => format!("F{n}"),
        Key::Left => token("Left"),
        Key::Right => token("Right"),
        Key::Up => token("Up"),
        Key::Down => token("Down"),
        Key::Backspace => token("Back"),
        Key::Enter => token("Enter"),
        Key::Esc => token("Esc"),
        Key::Tab => token("Tab"),
        Key::Space => token("Space"),
    });
    out
}

// keycaps for each label with a colored edge underneath
fn caps(labels: &[String], edge: Color) -> [Line<'static>; 2] {
    let (mut top, mut bottom) = (Vec::new(), Vec::new());
    for l in labels {
        let w = l.chars().count() + 2;
        top.push(Span::styled(
            format!(" {l} "),
            Style::new().bg(super::KEYCAP),
        ));
        top.push(Span::raw("  "));
        bottom.push(Span::styled("\u{2594}".repeat(w), Style::new().fg(edge)));
        bottom.push(Span::raw("  "));
    }
    [Line::from(top), Line::from(bottom)]
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let [_, deck, _, action, _, keys, _, status, _, footer] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);
    let card = CARDS[app.card];
    let answer = combo_labels(card.combo);
    let center = |l: Vec<Line<'static>>| {
        Paragraph::new(l)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
    };
    let recall = if card.recall { "   recall card" } else { "" };
    f.render_widget(
        center(vec![Line::styled(
            format!("{}{recall}", card.deck.label()),
            dim(),
        )]),
        deck,
    );
    f.render_widget(center(vec![Line::styled(card.action, bold())]), action);

    let neutral = Color::Gray;
    let (key_lines, status_line): (Vec<Line>, Line) = match &app.card_state {
        CardState::Ask if card.recall => (
            vec![Line::styled("think of the keys, then press space", dim())],
            Line::raw(""),
        ),
        CardState::Ask if app.seq_pos > 0 => {
            let steps = crate::cards::parse_seq(card.combo).map_or(0, |s| s.len());
            (
                vec![Line::styled("press the shortcut", dim())],
                Line::styled(
                    format!("keep going: {} of {steps} keys", app.seq_pos),
                    dim(),
                ),
            )
        }
        CardState::Ask => (
            vec![Line::styled("press the shortcut", dim())],
            Line::styled("space shows the answer", dim()),
        ),
        CardState::Judge => (
            caps(&answer, neutral).to_vec(),
            Line::styled("did you know it?    y yes    n no", dim()),
        ),
        CardState::Retry(Some(p)) => (
            caps(&answer, neutral).to_vec(),
            Line::from(vec![
                Span::styled("you pressed  ", dim()),
                Span::styled(pressed_labels(p).join(" "), Style::new().fg(super::BAD)),
                Span::styled("    now press it for real", dim()),
            ]),
        ),
        CardState::Retry(None) => (
            caps(&answer, neutral).to_vec(),
            Line::styled("now press it for real to lock it in", dim()),
        ),
        CardState::Done(_) => (
            caps(&answer, super::GOOD).to_vec(),
            Line::styled("correct", Style::new().fg(super::GOOD)),
        ),
    };
    f.render_widget(center(key_lines), keys);
    f.render_widget(center(vec![status_line]), status);

    let (k, n) = match app.deck {
        Some(d) => known(CARDS, &app.progress.boxes, d),
        None => crate::cards::Deck::ALL.iter().fold((0, 0), |(a, b), d| {
            let (x, y) = known(CARDS, &app.progress.boxes, *d);
            (a + x, b + y)
        }),
    };
    let name = app.deck.map_or("all decks", |d| d.label());
    f.render_widget(
        center(vec![Line::styled(
            format!("{name}   {k}/{n} learned"),
            dim(),
        )]),
        footer,
    );
}
