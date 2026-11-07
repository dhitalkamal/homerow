// what keys do on the shortcuts tab: grade the card, reveal, judge, change deck
use super::{App, CardState, DONE_PAUSE_MS, Input};
use crate::cards::{self, Deck, Key, Mods};
use crate::content::CARDS;

impl App {
    pub(super) fn card_input(&mut self, input: Input, now: u64) {
        let card = CARDS[self.card];
        if let Input::Text(b @ ('[' | ']')) = input
            && self.card_state != CardState::Judge
        {
            return self.cycle_deck(b == ']');
        }
        match (&self.card_state, input) {
            (CardState::Judge, Input::Text(ch @ ('y' | 'n'))) => {
                cards::grade(&mut self.progress.boxes, &card, ch == 'y');
                self.save();
                self.next_card();
            }
            (CardState::Judge | CardState::Done(_), _) => {}
            (CardState::Ask, Input::Text(' ')) if card.recall => self.card_state = CardState::Judge,
            (CardState::Ask, Input::Text(' ')) => {
                cards::grade(&mut self.progress.boxes, &card, false);
                self.save();
                self.card_state = CardState::Retry(None);
            }
            _ if card.recall => {}
            (state, _) => {
                let first_try = *state == CardState::Ask;
                let pressed = match input {
                    Input::Combo(c) => c,
                    Input::Text('\n') => cards::normalize(Mods::default(), Key::Enter),
                    Input::Text(' ') => cards::normalize(Mods::default(), Key::Space),
                    Input::Text(ch) => cards::normalize(Mods::default(), Key::Char(ch)),
                    _ => return,
                };
                let hit = cards::parse(card.combo) == Some(pressed);
                if first_try {
                    cards::grade(&mut self.progress.boxes, &card, hit);
                    self.save();
                }
                self.card_state = if hit {
                    CardState::Done(now + DONE_PAUSE_MS)
                } else {
                    CardState::Retry(Some(pressed))
                };
            }
        }
    }

    fn cycle_deck(&mut self, forward: bool) {
        let opts: Vec<Option<Deck>> = std::iter::once(None)
            .chain(Deck::ALL.iter().copied().map(Some))
            .collect();
        let i = opts.iter().position(|d| *d == self.deck).unwrap_or(0);
        let n = opts.len();
        self.deck = opts[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }];
        self.next_card();
    }
}
