mod app;
mod baseline;
mod brand;
mod cards;
mod cli;
mod clipboard;
mod content;
mod daily;
mod demo;
mod history;
mod input;
mod lessons;
mod rng;
mod session;
mod settings;
mod share;
mod stats;
mod store;
mod ui;
mod zoom;

use ratatui::crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::style::Print;
use ratatui::crossterm::{cursor, execute, terminal};
use std::io::Write;
use std::io::{self, stdout};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

const HELP: &str = "homerow - typing and mac shortcut trainer

usage: homerow [--demo] [--file PATH]

  --demo        start with two weeks of sample progress, kept in a temp file.
                your real progress is not read or changed.
  -f, --file    type your own text: the code tab uses PATH, a few lines at a
                time.

keys: tab switches mode, esc quits.
progress: ~/.local/share/homerow/progress.json";

fn main() -> io::Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = match cli::parse(&raw) {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    // load your own file before the screen takes over, so errors are readable
    let own = match &args.file {
        None => Vec::new(),
        Some(p) => match std::fs::read_to_string(p) {
            Ok(text) => {
                let chunks = content::own_text(&text);
                if chunks.is_empty() {
                    eprintln!(
                        "{} has no lines homerow can use (blank, or characters a us keyboard cannot type)",
                        p.display()
                    );
                    std::process::exit(2);
                }
                chunks
            }
            Err(e) => {
                eprintln!("could not read {}: {e}", p.display());
                std::process::exit(2);
            }
        },
    };
    let path = if args.demo {
        let p = std::env::temp_dir().join(format!("homerow-demo-{}.json", std::process::id()));
        store::save(&p, &demo::progress(now_ms()))?;
        p
    } else {
        store::default_path()
    };
    let mut app = app::App::new(path.clone(), now_ms());
    if let Some(p) = &args.file {
        app.own = own;
        app.own_name = p.file_name().map(|n| n.to_string_lossy().into_owned());
    }
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
    app.zoom_ok = probe_zoom();
    // the probe leaves a 2x space behind, clear it before the first frame
    term.clear()?;
    let result = run(&mut term, &mut app);
    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    if !args.demo {
        println!("progress saved to {}", path.display());
    } else {
        // demo data is throwaway, clean it up
        let _ = std::fs::remove_file(&path);
        println!("demo closed, your real progress was not touched");
    }
    result
}

// kitty text sizing check: a terminal that understands it moves the cursor
// two cells for a 2x space. any error (no reply, old terminal) means no zoom.
fn probe_zoom() -> bool {
    let mut out = stdout();
    let mut check = || -> io::Result<bool> {
        execute!(out, Print("\r"))?;
        let (before, _) = cursor::position()?;
        execute!(out, Print("\u{1b}]66;s=2; \u{7}"))?;
        let (after, _) = cursor::position()?;
        Ok(zoom::supported(before, after))
    };
    let ok = check().unwrap_or(false);
    let _ = execute!(stdout(), Print("\r\u{1b}[2K"));
    ok
}

fn run(term: &mut ratatui::DefaultTerminal, app: &mut app::App) -> io::Result<()> {
    // rows that held zoomed text last frame; when they change the screen is
    // cleared, since zoomed letters are not part of ratatui's buffer
    let mut last_rows: Vec<(u16, u16, u8)> = Vec::new();
    while !app.quit {
        let now = now_ms();
        if let Ok(size) = term.size() {
            app.screen = (size.width, size.height);
        }
        app.tick(now);
        let mut runs = Vec::new();
        term.draw(|f| ui::draw(f, app, now, &mut runs))?;
        let rows = zoom_rows(&runs);
        if rows != last_rows && !last_rows.is_empty() {
            term.clear()?;
            runs.clear();
            term.draw(|f| ui::draw(f, app, now, &mut runs))?;
        }
        if !runs.is_empty() {
            let mut out = stdout();
            out.write_all(zoom::encode(&runs).as_bytes())?;
            out.flush()?;
        }
        last_rows = rows;
        // short poll so the baseline clock and live wpm keep moving
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(k) = event::read()?
            && let Some(i) = input::map(k)
        {
            app.handle(i, now_ms());
            if let Some(card) = app.pending_copy.take() {
                let msg = match clipboard::copy(&card) {
                    clipboard::Copied::Tool(_) => "copied share card",
                    clipboard::Copied::Terminal => "share card sent to the terminal clipboard",
                };
                app.notify(msg, now_ms());
            }
        }
    }
    Ok(())
}

// start of each zoomed row, used to notice when the zoomed area moves
fn zoom_rows(runs: &[zoom::Run]) -> Vec<(u16, u16, u8)> {
    let mut rows: Vec<(u16, u16, u8)> = runs.iter().map(|r| (r.y, r.x, r.scale)).collect();
    rows.sort_unstable();
    rows.dedup_by_key(|r| r.0);
    rows
}
