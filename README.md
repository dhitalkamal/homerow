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
- progress: one view per mode (left/right switches between all modes,
  baseline, weak keys and code). Each view shows where you are now (average of
  the last 3 rounds), first round to now, best speed, accuracy, a speed trend
  (one line per mode) and that mode's weak keys. Code tracks symbols and digits
  too. Shortcut deck progress sits underneath.

Speed and key stats are tracked separately per mode, because code is slower
than prose and should not drag your baseline down. The weak keys drill uses
baseline and weak keys stats together. The results screen after a round only
compares with earlier rounds of the same mode.
- settings: switch the on-screen keyboard and hands on or off, plus finger
  colors, weak key heatmap, hidden key labels and size. Up/down (or j/k) move,
  space or return changes a row, left/right set it. A live preview shows the
  result. Everything here is saved.

While typing the screen only shows the text and one stats line. The keyboard
is off by default; turn it on in settings. With it on, two hands under the
keyboard light the finger for the next key (and the other pinky for shift).
Results appear in big digits after each round. A wrong key turns red and the
cursor waits until you press the right one.

Text and keycaps use your terminal's own colors, so they follow your theme.
Each letter on the keyboard is tinted by the finger that presses it (8 fixed
colors, so no theme can merge two fingers), the next key fills with that color,
and the line under the keyboard names the finger.

## big text

Off by default. Turn on the "big text" row on the settings tab to draw the
typing text in block letters, about four times the normal size (handy on a TV
or from a distance). They are made of ordinary block characters, so this works
in any terminal. It only applies when at least 24 letters fit on a line, and
falls back to normal text otherwise.

For a smaller step up, zoom your terminal instead (cmd+= in most macOS
terminals, ctrl+= or ctrl+shift+= elsewhere); the layout adapts.

## zoom (kitty)

In kitty 0.40 or newer, ctrl+] zooms the typing text in and ctrl+[ zooms it
out: normal, 2x, 3x. Only the text you type grows; the rest of the screen and
your other terminal windows stay as they are. It uses kitty's text sizing
protocol, so homerow checks for support at startup; in other terminals the
keys only show a short note and the settings row reads "needs kitty".

Careful outside kitty: many terminals send ctrl+[ as the esc key, and esc
quits homerow. In kitty the two keys stay separate.

## size and terminal zoom

Terminal zoom (whole window) is your terminal's job: in kitty, cmd+= and cmd+- change the font size.
homerow redraws to fit whatever size it gets:

- plenty of room: full keyboard and hands
- a bit less: full keyboard, hands hidden
- medium: compact keyboard (no blank lines between key rows)
- small or narrow: text and stats only

The density follows the window too. By default it is auto:

- below 80 columns or 20 rows: compact (text only, no line spacing)
- from 110 columns: spacious (wider text, an extra row)
- from 140 columns: large (110 column text, a keyboard with double width,
  two line keycaps and bigger hands)
- in between: comfortable

Auto looks mostly at width; when the window is short the hands and keyboard
step down on their own.

To pick one yourself, use ctrl+= and ctrl+- (they step from whatever auto is
showing) or the size row on the settings tab, where space cycles auto,
compact, comfortable, spacious and large. Your choice is saved.

ctrl+= needs a terminal that reports ctrl with symbol keys (kitty, WezTerm,
Ghostty with the kitty keyboard protocol). ctrl and the + key works too.

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
