// the settings tab's rows and what each key does to them. no drawing here.
use crate::baseline::Length;
use crate::store::{Density, Progress};

// kitty zoom steps: 1x (normal), 2x, 3x
pub const MAX_ZOOM: u8 = 3;
// daily goal: space cycles these, left/right step by 5 up to MAX_GOAL
const GOAL_PRESETS: [u32; 6] = [10, 15, 20, 30, 45, 60];
pub const MAX_GOAL: u32 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Keyboard,
    Hands,
    FingerColors,
    Heatmap,
    HideLabels,
    BigText,
    Zoom,
    Size,
    Goal,
    TestLength,
    Punctuation,
    Numbers,
    Ghost,
}

pub const ITEMS: [Item; 13] = [
    Item::Keyboard,
    Item::Hands,
    Item::FingerColors,
    Item::Heatmap,
    Item::HideLabels,
    Item::BigText,
    Item::Zoom,
    Item::Size,
    Item::Goal,
    Item::TestLength,
    Item::Punctuation,
    Item::Numbers,
    Item::Ghost,
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
            Item::Zoom => "zoom (kitty)",
            Item::Size => "size",
            Item::Goal => "daily goal",
            Item::TestLength => "test length",
            Item::Punctuation => "punctuation",
            Item::Numbers => "numbers",
            Item::Ghost => "race your best",
        }
    }

    pub fn value(self, p: &Progress) -> String {
        if self == Item::Size {
            return p.density.label().to_string();
        }
        if self == Item::Zoom {
            return format!("{}x", p.settings.zoom.clamp(1, MAX_ZOOM));
        }
        if self == Item::TestLength {
            return match p.settings.test {
                Length::Time(s) => format!("{s} seconds"),
                Length::Words(w) => format!("{w} words"),
            };
        }
        if self == Item::Goal {
            return match p.settings.goal_min {
                0 => "off".to_string(),
                m => format!("{m} min"),
            };
        }
        let s = &p.settings;
        let on = match self {
            Item::Keyboard => s.keyboard,
            Item::Hands => s.hands,
            Item::FingerColors => s.finger_colors,
            Item::Heatmap => s.heatmap,
            Item::HideLabels => s.hide_labels,
            Item::BigText => s.big_text,
            Item::Punctuation => s.punctuation,
            Item::Numbers => s.numbers,
            Item::Ghost => s.ghost,
            Item::Zoom | Item::Size | Item::Goal | Item::TestLength => false,
        };
        if on { "on" } else { "off" }.to_string()
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
        Item::Punctuation => &mut s.punctuation,
        Item::Numbers => &mut s.numbers,
        Item::Ghost => &mut s.ghost,
        Item::Zoom | Item::Size | Item::Goal | Item::TestLength => return None,
    })
}

// space or return: flip a switch, or cycle the size round
pub fn toggle(item: Item, p: &mut Progress) {
    if item == Item::TestLength {
        p.settings.test = p.settings.test.step(true);
        return;
    }
    if item == Item::Goal {
        let g = p.settings.goal_min;
        p.settings.goal_min = GOAL_PRESETS.iter().copied().find(|m| *m > g).unwrap_or(0);
        return;
    }
    if item == Item::Zoom {
        p.settings.zoom = p.settings.zoom % MAX_ZOOM + 1;
        return;
    }
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
    if item == Item::TestLength {
        p.settings.test = p.settings.test.step(right);
        return;
    }
    if item == Item::Goal {
        let g = p.settings.goal_min;
        p.settings.goal_min = if right {
            (g + 5).min(MAX_GOAL)
        } else {
            g.saturating_sub(5)
        };
        return;
    }
    if item == Item::Zoom {
        p.settings.zoom = if right {
            (p.settings.zoom + 1).min(MAX_ZOOM)
        } else {
            p.settings.zoom.saturating_sub(1).max(1)
        };
        return;
    }
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
        assert_eq!(Item::BigText.value(&p), "off");
        toggle(Item::BigText, &mut p);
        assert!(p.settings.big_text);
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

    #[test]
    fn zoom_row_cycles_and_steps_between_1x_and_3x() {
        let mut p = Progress::default();
        assert_eq!(Item::Zoom.value(&p), "1x");
        toggle(Item::Zoom, &mut p);
        assert_eq!(Item::Zoom.value(&p), "2x");
        toggle(Item::Zoom, &mut p);
        toggle(Item::Zoom, &mut p);
        assert_eq!(p.settings.zoom, 1, "space wraps from 3x to 1x");
        adjust(Item::Zoom, &mut p, false, (100, 40));
        assert_eq!(p.settings.zoom, 1);
        for _ in 0..5 {
            adjust(Item::Zoom, &mut p, true, (100, 40));
        }
        assert_eq!(p.settings.zoom, 3);
    }

    #[test]
    fn daily_goal_row_cycles_presets_and_steps_by_five() {
        let mut p = Progress::default();
        assert_eq!(Item::Goal.value(&p), "15 min");
        toggle(Item::Goal, &mut p);
        assert_eq!(p.settings.goal_min, 20);
        adjust(Item::Goal, &mut p, true, (100, 40));
        assert_eq!(p.settings.goal_min, 25);
        for _ in 0..10 {
            adjust(Item::Goal, &mut p, false, (100, 40));
        }
        assert_eq!(p.settings.goal_min, 0);
        assert_eq!(Item::Goal.value(&p), "off");
        for _ in 0..40 {
            adjust(Item::Goal, &mut p, true, (100, 40));
        }
        assert_eq!(p.settings.goal_min, MAX_GOAL);
    }

    #[test]
    fn test_length_row_steps_through_the_lengths() {
        use crate::baseline::Length;
        let mut p = Progress::default();
        assert_eq!(Item::TestLength.value(&p), "60 seconds");
        toggle(Item::TestLength, &mut p);
        assert_eq!(p.settings.test, Length::Time(120));
        adjust(Item::TestLength, &mut p, true, (100, 40));
        assert_eq!(Item::TestLength.value(&p), "25 words");
        toggle(Item::Punctuation, &mut p);
        assert!(p.settings.punctuation);
    }

    #[test]
    fn ghost_is_on_by_default_and_can_be_switched_off() {
        let mut p = Progress::default();
        assert_eq!(Item::Ghost.value(&p), "on");
        toggle(Item::Ghost, &mut p);
        assert!(!p.settings.ghost);
    }
}
