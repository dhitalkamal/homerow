// the settings tab's rows and what each key does to them. no drawing here.
use crate::store::{Density, Progress};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Keyboard,
    Hands,
    FingerColors,
    Heatmap,
    HideLabels,
    BigText,
    Size,
}

pub const ITEMS: [Item; 7] = [
    Item::Keyboard,
    Item::Hands,
    Item::FingerColors,
    Item::Heatmap,
    Item::HideLabels,
    Item::BigText,
    Item::Size,
];

impl Item {
    pub fn label(self) -> &'static str {
        match self {
            Item::Keyboard => "keyboard",
            Item::Hands => "hands",
            Item::FingerColors => "finger colors",
            Item::Heatmap => "weak key heatmap",
            Item::HideLabels => "hide key labels",
            Item::BigText => "big text",
            Item::Size => "size",
        }
    }

    pub fn value(self, p: &Progress) -> &'static str {
        if self == Item::Size {
            return p.density.label();
        }
        let s = &p.settings;
        let on = match self {
            Item::Keyboard => s.keyboard,
            Item::Hands => s.hands,
            Item::FingerColors => s.finger_colors,
            Item::Heatmap => s.heatmap,
            Item::HideLabels => s.hide_labels,
            Item::BigText => s.big_text,
            Item::Size => false,
        };
        if on { "on" } else { "off" }
    }
}

// the on/off flag behind a row, None for size
fn switch(item: Item, p: &mut Progress) -> Option<&mut bool> {
    let s = &mut p.settings;
    Some(match item {
        Item::Keyboard => &mut s.keyboard,
        Item::Hands => &mut s.hands,
        Item::FingerColors => &mut s.finger_colors,
        Item::Heatmap => &mut s.heatmap,
        Item::HideLabels => &mut s.hide_labels,
        Item::BigText => &mut s.big_text,
        Item::Size => return None,
    })
}

// space or return: flip a switch, or cycle the size round
pub fn toggle(item: Item, p: &mut Progress) {
    match switch(item, p) {
        Some(on) => *on = !*on,
        None => {
            p.density = match p.density {
                Density::Auto => Density::Compact,
                Density::Compact => Density::Comfortable,
                Density::Comfortable => Density::Spacious,
                Density::Spacious => Density::Large,
                Density::Large => Density::Auto,
            }
        }
    }
}

// right turns a switch on or makes size bigger, left the opposite.
// from auto, size steps from whatever auto shows at this terminal size.
pub fn adjust(item: Item, p: &mut Progress, right: bool, screen: (u16, u16)) {
    match switch(item, p) {
        Some(on) => *on = right,
        None => p.density = p.density.resolve(screen.0, screen.1).step(right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_switches() {
        let mut p = Progress::default();
        toggle(Item::Keyboard, &mut p);
        assert!(p.settings.keyboard);
        assert_eq!(Item::Keyboard.value(&p), "on");
        toggle(Item::Keyboard, &mut p);
        assert_eq!(Item::Keyboard.value(&p), "off");
        toggle(Item::HideLabels, &mut p);
        assert!(p.settings.hide_labels);
        assert_eq!(Item::BigText.value(&p), "on");
        toggle(Item::BigText, &mut p);
        assert!(!p.settings.big_text);
    }

    #[test]
    fn toggle_cycles_size_round_through_auto() {
        let mut p = Progress::default();
        assert_eq!(Item::Size.value(&p), "auto");
        let mut seen = Vec::new();
        for _ in 0..5 {
            toggle(Item::Size, &mut p);
            seen.push(p.density);
        }
        use Density::*;
        assert_eq!(seen, vec![Compact, Comfortable, Spacious, Large, Auto]);
    }

    #[test]
    fn left_and_right_set_switches_and_step_size() {
        let mut p = Progress::default();
        adjust(Item::Heatmap, &mut p, true, (100, 40));
        adjust(Item::Heatmap, &mut p, true, (100, 40));
        assert!(p.settings.heatmap, "right means on, not flip");
        adjust(Item::Heatmap, &mut p, false, (100, 40));
        assert!(!p.settings.heatmap);
        adjust(Item::Size, &mut p, false, (100, 40));
        adjust(Item::Size, &mut p, false, (100, 40));
        assert_eq!(p.density, Density::Compact, "size stops at the ends");
    }
}
