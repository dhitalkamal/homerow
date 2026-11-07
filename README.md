# homerow

Terminal trainer for touch typing and Mac shortcuts. One binary, no runtime
dependencies.

## install

    cargo install --path .

That puts `homerow` in `~/.cargo/bin`. Run it with `homerow`.

To preview with sample data (two weeks of made-up rounds, your real progress is
not read or changed):

    homerow --demo

## modes

Tab cycles through them, Esc quits.

- baseline: random common words for 60 seconds. Use it to track your WPM.
- weak keys: words built around your 3 slowest or most missed letters.
- code: short real snippets (Python, Go, Rust, TypeScript, SQL, shell, git).
  Indentation after Return is typed for you.
- shortcuts: flash cards for text editing, macOS, Finder, zsh and IntelliJ.
- progress: dashboard with latest and best speed, accuracy, day streak, a
  speed trend chart, weak keys and how far along each shortcut deck is.

While typing the screen only shows the text, one stats line and the keyboard.
Results appear in big digits after each round. A wrong key turns red and the
cursor waits until you press the right one.

Text and keycaps use your terminal's own colors, so they follow your theme.
Each letter on the keyboard is tinted by the finger that presses it (8 fixed
colors, so no theme can merge two fingers), the next key fills with that color,
and the line under the keyboard names the finger.

## size and zoom

Zoom is your terminal's job: in kitty, cmd+= and cmd+- change the font size.
homerow redraws to fit whatever size it gets:

- plenty of room: full keyboard
- medium: compact keyboard (no blank lines between key rows)
- small or narrow: text and stats only

On top of that, ctrl+= and ctrl+- change the density, saved between runs:

- compact: text only, no line spacing
- comfortable: the default layout
- spacious: wider text column and an extra row of text

ctrl+= needs a terminal that reports ctrl with symbol keys (kitty, WezTerm,
Ghostty with the kitty keyboard protocol). ctrl and the + key works too.

Typing keys: ctrl+F finger colors, ctrl+K weak key heatmap, ctrl+L hide key
labels (forces you to stop looking).

Shortcut keys: press the real combo. Space shows the answer (counts as a miss).
`[` and `]` change deck. A card is learned after 3 right answers in a row.

## recall cards

Some shortcuts never reach a terminal app because macOS or the terminal takes
them first (cmd+Tab, cmd+Space, cmd+Q, screenshots). Those are recall cards:
think of the keys, press Space, then y or n.

## terminal setup for cmd and opt

- cmd: needs a terminal that speaks the kitty keyboard protocol (kitty,
  WezTerm, Ghostty, recent iTerm2 with the CSI u option on). homerow turns it on
  when available. Without it, cmd cards still work through Space and y/n.
- opt: kitty sends special characters for opt+letter unless you set
  `macos_option_as_alt both` in kitty.conf. Arrows and delete work either way.
- Any combo your terminal binds itself (kitty uses cmd+t, cmd+w, cmd+c and
  more) never reaches homerow. Use Space to reveal those.

## progress

Saved to `~/.local/share/homerow/progress.json` (or `$XDG_DATA_HOME/homerow`)
after every round and card. An unreadable file is moved aside as
`progress.json.bad-<time>`, never deleted.

## dev

    cargo test
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check

Logic lives in pure modules with tests (stats, session, cards, content, store,
app, input). `ui/` only draws, `main.rs` runs the event loop.
