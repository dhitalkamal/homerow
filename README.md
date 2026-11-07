# homerow

Terminal trainer for touch typing and Mac shortcuts. One binary, no runtime
dependencies.

## install

One line, on macOS or Linux:

    curl -fsSL https://raw.githubusercontent.com/dhitalkamal/homerow/main/install.sh | sh

On Apple Silicon Macs this downloads the latest release, checks it against the
published SHA-256 checksum, removes the download quarantine flag (the binary is
not Apple signed) and installs it to `~/.local/bin`. On other systems it builds
from source with `cargo`, or tells you how to get Rust first. It never uses
sudo, and installs nothing if the download or checksum is wrong. Run the same
line again to update.

If `~/.local/bin` is not on your PATH yet, the installer adds it to your shell's
startup file (zsh: `~/.zshrc`, bash: `~/.bash_profile` on macOS or `~/.bashrc`
on Linux, fish: `config.fish`), once, with a comment saying it came from the
homerow installer, and tells you which file it changed. Other shells get the
line printed instead. `HOMEROW_NO_MODIFY_PATH=1` skips this step.

Options: `HOMEROW_VERSION=v0.1.0` pins a release, `HOMEROW_INSTALL_DIR=...`
picks another folder. The checksum catches broken or changed downloads; it
comes from the same GitHub release, so it does not protect against a
compromised GitHub account. Read `install.sh` first if you like, it is short.

### Manual download (macOS, Apple Silicon)

    curl -L https://github.com/dhitalkamal/homerow/releases/latest/download/homerow-aarch64-apple-darwin.tar.gz | tar xz
    xattr -d com.apple.quarantine homerow 2>/dev/null
    mkdir -p ~/.local/bin && mv homerow ~/.local/bin/

### Any platform, from source

With Rust installed (https://rustup.rs):

    cargo install --git https://github.com/dhitalkamal/homerow

That puts `homerow` in `~/.cargo/bin`. Works on macOS, Linux and Windows
terminals.

To preview with sample data (two weeks of made-up rounds, your real progress is
not read or changed):

    homerow --demo

To type your own text, pass a file; the code tab then uses it, a few lines at
a time:

    homerow --file src/main.rs

Tabs become 4 spaces, shared indentation is removed, control characters are
stripped (a file cannot send escape codes to your terminal), and lines with
characters a US keyboard cannot type are skipped.

## modes

Tab cycles through them, Esc quits.

- baseline: a typing test on common words. On the settings tab pick the length
  (15, 30, 60 or 120 seconds, or 25, 50 or 100 words) and switch on punctuation
  (sentences with capitals, commas and full stops) or numbers. Each round is
  filed under its test, like "30s +punct", and only compared with the same test.

Race your best: once you have a best for a mode and test, a ghost runs through
the text at that pace (a gray block on the letter it has reached) and the stats
line shows "ghost 41: +6" when you are ahead or "-4" when behind. Switch it off
with the "race your best" row on the settings tab.
- weak keys: words built around your 3 slowest or most missed letters and your
  2 slowest letter pairs (like "ng" or "er"), shown on the stats line.
- code: short real snippets (Python, Go, Rust, TypeScript, SQL, shell, git).
  Indentation after Return is typed for you.
- mistakes: drills the words you got wrong in the other modes, weighted by how
  often you missed them. Typing a word cleanly (in any mode) counts it down, so
  fixed words drop off the list.
- lessons: learn touch typing key by key. You start with the home row
  (f j d k s l a ;); a lesson round with 95% accuracy and 20 wpm unlocks the
  next key (g h e i r u t n o c w m v b p y x q z). Lessons use only keys you
  have unlocked, as letter groups at first and real words once there are
  enough, with the newest key coming up often. The keyboard and hands always
  show here so you can see where the new key is.
- daily: the daily challenge. The same 50 words for everyone on the same
  version that day (picked from the UTC date, no server), so you can compare
  wpm with friends. Retry as often as you like; the stats line shows your best
  today, and the ghost races it.
- shortcuts: flash cards for text editing, macOS, Finder, zsh, IntelliJ,
  VS Code, Chrome / Arc, tmux (default ctrl+b prefix) and vim. Key sequences
  like tmux's ctrl+b then c, or vim's g g and c i w, are graded step by step
  ("keep going: 1 of 2 keys"); a wrong step starts the card over.
- progress: one view per mode (left/right switches between all modes,
  baseline, weak keys and code). Each view shows where you are now (average of
  the last 3 rounds), first round to now, best speed, accuracy, a speed trend
  (one line per mode) and that mode's weak keys. Code tracks symbols and digits
  too, and the text views list your slowest letter pairs. Shortcut deck
  progress sits underneath.
  Return opens the round history for the current view: every round with when,
  mode, wpm and accuracy, newest first, scrolled with up/down.

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

## share card

On any results screen press c to copy a short card for friends:

    homerow daily 2026-11-07: 52 wpm, 97% accuracy
    streak 4 days, try it: github.com/dhitalkamal/homerow

It uses your system clipboard tool (pbcopy on macOS, wl-copy, xclip or xsel on
Linux, clip on Windows), run directly without a shell. Without one it falls
back to the terminal clipboard code (OSC 52), which kitty, iTerm2, WezTerm and
many others support.

## github badge

Show your typing speed on your GitHub profile:

    homerow login

homerow shows a one-time code, opens github.com/login/device when you press
Enter, and waits while you approve it there. It asks for the `gist` scope only
(create and edit your gists, nothing else on your account), makes one secret
gist that holds the badge, and prints a line for your profile README:

    [![typing speed](https://img.shields.io/endpoint?url=...)](https://github.com/dhitalkamal/homerow)

The badge shows your recent baseline speed, the average of your last 3
baseline rounds, like "typing 52 wpm". It updates when you quit homerow, and
only when the number changed. A quit without internet stays silent and tries
again next time. `homerow sync` updates it right away and says what went wrong
if something did. GitHub and shields.io cache the badge for a few minutes.

The GitHub token is kept in your system keychain (macOS Keychain, Windows
Credential Manager, or the Secret Service on Linux, such as GNOME Keyring or
KWallet), never in a file. `github.json` next to your progress file only says
which gist holds the badge. On macOS the keychain may ask for permission again
after homerow is updated, since the binary is not Apple signed; Always Allow
stops the question until the next update.

`homerow logout` removes the token and that file. The gist stays on GitHub
(the logout message links to it, delete it there if you like), and
https://github.com/settings/applications revokes homerow's access completely.
Secret gists are unlisted, not private: anyone with the link can see the badge
file, which is what a badge is for.

## daily goal

The "daily goal" row on the settings tab sets how many minutes a day you want to
practice (15 by default; space cycles 10, 15, 20, 30, 45, 60, left/right step by
5, 0 turns it off). The progress tab and the results screen show today against
the goal, and the day streak only counts days that met it.

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

## your own decks

Write your own shortcuts in `~/.config/homerow/decks.txt` (or pass
`--decks PATH`) and they show up as decks on the shortcuts tab and the
progress tab:

    # lines starting with # are comments
    [Figma]
    Duplicate selection = cmd+D
    Group selection = cmd+G
    Zoom to fit = shift+1

    [tmux]
    New window = ctrl+a c

Keys use the same names as the built-in cards (cmd, opt, ctrl, shift, Left,
Up, Back, Enter, Esc, Tab, Space, F1-F12); separate steps with spaces for
sequences like `ctrl+a c`. Keys a terminal never sees become recall cards.
Bad lines are skipped and the first one is shown at the bottom of the screen.

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
app, input, github, account). account.rs runs the github flows against small
traits, so tests use fakes and never touch the network or your keychain. `ui/` only draws, `main.rs` runs the event loop.

## license

MIT, see LICENSE.
