// the shortcut card in front of you, plus how far along each deck is
use crate::app::{App, CardState};
use crate::cards::{Combo, Deck, Key, known};
use crate::content::CARDS;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

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

fn caps(labels: Vec<String>, color: Color) -> Line<'static> {
    let mut spans = Vec::new();
    for l in labels {
        spans.push(Span::styled(
            format!(" {l} "),
            Style::new()
                .fg(Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw("  "));
    }
    Line::from(spans)
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let [card_area, deck_area] =
        Layout::vertical([Constraint::Min(7), Constraint::Length(8)]).areas(area);
    let card = CARDS[app.card];
    let answer = combo_labels(card.combo);
    let dim = Style::new().fg(super::DIM);
    let mut lines = vec![
        Line::styled(
            format!(
                "{}{}",
                card.deck.label(),
                if card.recall { "  /  recall card" } else { "" }
            ),
            dim,
        ),
        Line::raw(""),
        Line::styled(card.action, Style::new().add_modifier(Modifier::BOLD)),
        Line::raw(""),
    ];
    match &app.card_state {
        CardState::Ask if card.recall => lines.push(Line::styled(
            "Think of the keys, then press Space to check.",
            dim,
        )),
        CardState::Ask => lines.push(Line::styled(
            "Press the shortcut. Space shows the answer.",
            dim,
        )),
        CardState::Judge => {
            lines.push(caps(answer, Color::White));
            lines.push(Line::raw(""));
            lines.push(Line::styled("Did you know it?  y = yes   n = no", dim));
        }
        CardState::Retry(pressed) => {
            if let Some(p) = pressed {
                lines.push(Line::from(vec![Span::styled("you pressed  ", dim)]));
                lines.push(caps(pressed_labels(p), super::BAD));
            }
            lines.push(Line::styled("answer", dim));
            lines.push(caps(answer, Color::White));
            lines.push(Line::styled("Now press it for real to lock it in.", dim));
        }
        CardState::Done(_) => {
            lines.push(caps(answer, super::GOOD));
            lines.push(Line::styled("Correct", Style::new().fg(super::GOOD)));
        }
    }
    let block = Block::new().borders(Borders::ALL).border_style(dim);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        card_area,
    );

    let decks: Vec<Line> = Deck::ALL
        .iter()
        .map(|&d| {
            let (k, n) = known(CARDS, &app.progress.boxes, d);
            let filled = (k * 20).checked_div(n).unwrap_or(0);
            let marker = if app.deck == Some(d) { "> " } else { "  " };
            Line::from(vec![
                Span::raw(format!("{marker}{:<16}", d.label())),
                Span::styled("#".repeat(filled), Style::new().fg(super::GOOD)),
                Span::styled("-".repeat(20 - filled), dim),
                Span::styled(format!("  {k}/{n} learned"), dim),
            ])
        })
        .collect();
    let title = if app.deck.is_none() {
        " Decks (all) "
    } else {
        " Decks "
    };
    f.render_widget(
        Paragraph::new(decks).block(
            Block::new()
                .title(title)
                .borders(Borders::TOP)
                .border_style(dim),
        ),
        deck_area,
    );
}
