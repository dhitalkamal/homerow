mod app;
mod cards;
mod content;
mod input;
mod rng;
mod session;
mod stats;
mod store;
mod ui;

use ratatui::crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{execute, terminal};
use std::io::{self, stdout};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn main() -> io::Result<()> {
    let path = store::default_path();
    let mut app = app::App::new(path.clone(), now_ms());
    let mut term = ratatui::init();
    // kitty keyboard protocol: lets cmd (super) combos reach us. terminals
    // without it still work, they just can't report cmd.
    let enhanced = terminal::supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }
    let result = run(&mut term, &mut app);
    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    println!("progress saved to {}", path.display());
    result
}

fn run(term: &mut ratatui::DefaultTerminal, app: &mut app::App) -> io::Result<()> {
    while !app.quit {
        let now = now_ms();
        app.tick(now);
        term.draw(|f| ui::draw(f, app, now))?;
        // short poll so the baseline clock and live wpm keep moving
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(k) = event::read()?
            && let Some(i) = input::map(k)
        {
            app.handle(i, now_ms());
        }
    }
    Ok(())
}
