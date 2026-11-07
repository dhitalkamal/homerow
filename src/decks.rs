// the built-in shortcut cards, grouped by deck
use crate::cards::{Card, Deck};

const fn c(deck: Deck, action: &'static str, combo: &'static str) -> Card {
    Card {
        deck,
        action,
        combo,
        recall: false,
    }
}
// recall-only: macOS or the terminal itself takes these keys first
const fn r(deck: Deck, action: &'static str, combo: &'static str) -> Card {
    Card {
        deck,
        action,
        combo,
        recall: true,
    }
}
use Deck::*;
pub const CARDS: &[Card] = &[
    c(Text, "Jump to start of line", "cmd+Left"),
    c(Text, "Jump to end of line", "cmd+Right"),
    c(Text, "Move back one word", "opt+Left"),
    c(Text, "Move forward one word", "opt+Right"),
    c(Text, "Jump to top of document", "cmd+Up"),
    c(Text, "Select to start of line", "cmd+shift+Left"),
    c(Text, "Select the next word", "opt+shift+Right"),
    c(Text, "Delete previous word", "opt+Back"),
    c(Text, "Delete to start of line", "cmd+Back"),
    c(Text, "Redo", "cmd+shift+Z"),
    c(
        Text,
        "Start of line, Emacs style (most Mac text fields)",
        "ctrl+A",
    ),
    c(Text, "Delete to end of line, Emacs style", "ctrl+K"),
    r(Text, "Forward delete one character", "fn+Back"),
    r(Mac, "Open Spotlight", "cmd+Space"),
    r(Mac, "Switch apps", "cmd+Tab"),
    c(Mac, "Cycle windows of the current app", "cmd+`"),
    r(Mac, "Hide current app", "cmd+H"),
    r(Mac, "Hide all other apps", "cmd+opt+H"),
    r(Mac, "Quit app", "cmd+Q"),
    r(Mac, "Close window", "cmd+W"),
    r(Mac, "Minimize window", "cmd+M"),
    r(Mac, "Open settings of the current app", "cmd+,"),
    r(Mac, "Screenshot an area", "cmd+shift+4"),
    r(Mac, "Screenshot and screen recording tools", "cmd+shift+5"),
    r(Mac, "Lock screen", "ctrl+cmd+Q"),
    r(Mac, "Force quit dialog", "cmd+opt+Esc"),
    r(Mac, "Mission Control", "ctrl+Up"),
    r(Mac, "Move to the desktop space on the right", "ctrl+Right"),
    r(Mac, "Emoji and symbols picker", "ctrl+cmd+Space"),
    c(Finder, "Go to folder by typing a path", "cmd+shift+G"),
    c(Finder, "Show or hide hidden files", "cmd+shift+."),
    r(Finder, "Quick Look the selected file", "Space"),
    c(Finder, "Open the parent folder", "cmd+Up"),
    c(Finder, "Open the selected item", "cmd+Down"),
    r(Finder, "New folder", "cmd+shift+N"),
    c(Finder, "Copy path of the selected file", "cmd+opt+C"),
    c(Finder, "Move file to Trash", "cmd+Back"),
    c(
        Finder,
        "Move a copied file here (cut and paste)",
        "cmd+opt+V",
    ),
    c(Zsh, "Start of command line", "ctrl+A"),
    c(Zsh, "End of command line", "ctrl+E"),
    c(Zsh, "Delete previous word", "ctrl+W"),
    c(Zsh, "Delete whole line", "ctrl+U"),
    c(Zsh, "Delete to end of line", "ctrl+K"),
    c(Zsh, "Search command history", "ctrl+R"),
    c(Zsh, "Clear the screen", "ctrl+L"),
    c(Zsh, "Paste back what you just deleted", "ctrl+Y"),
    c(Zsh, "Cancel the running command", "ctrl+C"),
    c(Zsh, "Previous command", "ctrl+P"),
    r(Idea, "Search everywhere", "shift shift"),
    c(Idea, "Find action by name", "cmd+shift+A"),
    c(Idea, "Recent files", "cmd+E"),
    c(Idea, "Go to declaration", "cmd+B"),
    c(Idea, "Show quick fixes", "opt+Enter"),
    c(Idea, "Reformat code", "cmd+opt+L"),
    c(Idea, "Rename symbol", "shift+F6"),
    c(Idea, "Duplicate line", "cmd+D"),
    c(Idea, "Delete line", "cmd+Back"),
    c(Idea, "Expand selection", "opt+Up"),
    c(Idea, "Toggle line comment", "cmd+/"),
    c(Idea, "Go to file", "cmd+shift+O"),
    c(Idea, "Extract variable", "cmd+opt+V"),
    c(Idea, "File structure popup", "cmd+F12"),
    c(Idea, "Find in files", "cmd+shift+F"),
    c(Idea, "Run", "ctrl+R"),
    // vs code on macos
    c(VsCode, "Quick open a file", "cmd+P"),
    c(VsCode, "Command palette", "cmd+shift+P"),
    c(VsCode, "Select the next match too", "cmd+D"),
    c(VsCode, "Toggle line comment", "cmd+/"),
    c(VsCode, "Move line up", "opt+Up"),
    c(VsCode, "Copy line down", "opt+shift+Down"),
    c(VsCode, "Toggle the sidebar", "cmd+B"),
    c(VsCode, "Toggle the terminal", "ctrl+`"),
    c(VsCode, "Search in all files", "cmd+shift+F"),
    c(VsCode, "Go to definition", "F12"),
    c(VsCode, "Rename symbol", "F2"),
    c(VsCode, "Delete line", "cmd+shift+K"),
    c(VsCode, "Keyboard shortcuts list", "cmd+K cmd+S"),
    c(VsCode, "Zen mode", "cmd+K z"),
    // chrome and arc on macos
    r(Browser, "New tab", "cmd+T"),
    r(Browser, "Close tab", "cmd+W"),
    c(Browser, "Reopen the tab you just closed", "cmd+shift+T"),
    c(Browser, "Jump to the address bar", "cmd+L"),
    c(Browser, "Next tab", "ctrl+Tab"),
    c(Browser, "First tab", "cmd+1"),
    c(Browser, "Reload", "cmd+R"),
    c(Browser, "Reload without cache", "cmd+shift+R"),
    c(Browser, "Developer tools", "cmd+opt+I"),
    c(Browser, "Find on the page", "cmd+F"),
    c(Browser, "Back", "cmd+["),
    c(Browser, "Arc: show or hide the sidebar", "cmd+S"),
    c(Browser, "Arc: copy the page link", "cmd+shift+C"),
    // tmux with the default ctrl+b prefix
    c(Tmux, "New window", "ctrl+b c"),
    c(Tmux, "Next window", "ctrl+b n"),
    c(Tmux, "Previous window", "ctrl+b p"),
    c(Tmux, "Split side by side", "ctrl+b %"),
    c(Tmux, "Split top and bottom", "ctrl+b \""),
    c(Tmux, "Detach", "ctrl+b d"),
    c(Tmux, "Zoom the pane", "ctrl+b z"),
    c(Tmux, "Rename the window", "ctrl+b ,"),
    c(Tmux, "Copy mode", "ctrl+b ["),
    c(Tmux, "Go to the next pane", "ctrl+b o"),
    c(Tmux, "Close the pane", "ctrl+b x"),
    // vim normal mode
    c(Vim, "Delete the line", "d d"),
    c(Vim, "Copy (yank) the line", "y y"),
    c(Vim, "Paste after the cursor", "p"),
    c(Vim, "Undo", "u"),
    c(Vim, "Redo", "ctrl+R"),
    c(Vim, "Top of the file", "g g"),
    c(Vim, "End of the file", "shift+G"),
    c(Vim, "Start of the line", "0"),
    c(Vim, "End of the line", "$"),
    c(Vim, "Next word", "w"),
    c(Vim, "Back a word", "b"),
    c(Vim, "Change the word under the cursor", "c i w"),
    c(Vim, "Open a line below", "o"),
    c(Vim, "Open a line above", "shift+O"),
    c(Vim, "Add at the end of the line", "shift+A"),
    c(Vim, "Delete the character", "x"),
    c(Vim, "Search", "/"),
    c(Vim, "Next match", "n"),
    c(Vim, "Jump to the matching bracket", "%"),
    c(Vim, "Save and quit", ": w q Enter"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::parse_seq;

    #[test]
    fn every_live_card_parses_and_every_unparseable_card_is_recall() {
        for card in CARDS.iter() {
            if parse_seq(card.combo).is_none() {
                assert!(card.recall, "{} needs to be a recall card", card.combo);
            }
        }
    }

    #[test]
    fn card_ids_are_unique() {
        let mut ids: Vec<String> = CARDS.iter().map(|c| c.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CARDS.len());
    }

    #[test]
    fn new_decks_have_cards() {
        for d in [Deck::VsCode, Deck::Browser, Deck::Tmux, Deck::Vim] {
            assert!(CARDS.iter().filter(|c| c.deck == d).count() >= 8, "{d:?}");
        }
    }
}
