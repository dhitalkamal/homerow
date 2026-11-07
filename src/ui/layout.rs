// how the typing screen fits a given body size: text rows, width, big
// letters, keyboard size and hands. pure, so every size can be tested.
use super::{hands, keyboard};
use crate::store::Density;

// the keyboard needs this many columns, narrower windows drop it
const KEYBOARD_MIN_WIDTH: u16 = 62;
const LARGE_KEYBOARD_MIN_WIDTH: u16 = 122;
// big text: each letter is 4 columns, each row 4 lines plus a blank line
const BIG_LETTER: u16 = 4;
pub const BIG_ROW: u16 = 5;
const BIG_MIN_LETTERS: u16 = 24;
const BIG_MAX_WIDTH: u16 = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kbd {
    // double width keys with 2 line keycaps, for very big terminals
    Large,
    Full,
    Compact,
    Hidden,
}

// how the typing screen fits a body area of this size at this density
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub rows: u16,
    // a blank line after each text row
    pub spaced: bool,
    pub text_width: u16,
    pub kbd: Kbd,
    // finger bars under the keyboard
    pub hands: bool,
    // block letter text, each row 4 lines tall plus a blank line
    pub big: bool,
}

// keyboard, hands and big text come from settings; each only shows when it fits
pub fn plan(
    width: u16,
    height: u16,
    density: Density,
    keyboard: bool,
    hands: bool,
    big: bool,
) -> Plan {
    let (mut rows, spaced, w) = match density {
        Density::Compact => (3, false, 60),
        Density::Comfortable | Density::Auto => (3, true, 70),
        Density::Spacious => (4, true, 84),
        Density::Large => (4, true, 110),
    };
    // big text: as many letters as fit, up to 40, if at least 24 fit
    let big_width = width.saturating_sub(4).min(BIG_MAX_WIDTH);
    let mut big_rows = if keyboard { 2 } else { 3 };
    while big_rows > 0 && big_rows * BIG_ROW + 1 > height {
        big_rows -= 1;
    }
    let big = big && big_width / BIG_LETTER >= BIG_MIN_LETTERS && big_rows > 0;
    let (spaced, w) = if big { (true, big_width) } else { (spaced, w) };
    if big {
        rows = big_rows;
    }
    let per = if big {
        BIG_ROW
    } else if spaced {
        2
    } else {
        1
    };
    // the stats line, plus a blank above it when rows are not spaced
    let fixed = if spaced { 1 } else { 2 };
    while rows > 1 && rows * per + fixed > height {
        rows -= 1;
    }
    // text block, stats, and a 2 line gap above the keyboard
    let needed = rows * per + fixed + 2;
    let kbd = if !keyboard || density == Density::Compact || width < KEYBOARD_MIN_WIDTH {
        Kbd::Hidden
    } else if density == Density::Large
        && width >= LARGE_KEYBOARD_MIN_WIDTH
        && height >= needed + keyboard::LARGE_HEIGHT + 2
    {
        Kbd::Large
    } else if height >= needed + keyboard::FULL_HEIGHT + 2 {
        Kbd::Full
    } else if height >= needed + keyboard::COMPACT_HEIGHT + 2 {
        Kbd::Compact
    } else {
        Kbd::Hidden
    };
    let kbd_h = match kbd {
        Kbd::Large => keyboard::LARGE_HEIGHT,
        Kbd::Full => keyboard::FULL_HEIGHT,
        Kbd::Compact => keyboard::COMPACT_HEIGHT,
        Kbd::Hidden => 0,
    };
    // hands go first when space runs out, the keyboard is more useful
    let hands = hands && kbd != Kbd::Hidden && height >= needed + kbd_h + hands::HEIGHT + 1 + 2;
    Plan {
        rows,
        spaced,
        text_width: w.min(width),
        kbd,
        hands,
        big,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Density::*;

    fn on(w: u16, h: u16, d: Density) -> Plan {
        plan(w, h, d, true, true, false)
    }

    #[test]
    fn keyboard_off_means_text_only() {
        let p = plan(120, 50, Comfortable, false, true, false);
        assert_eq!((p.kbd, p.hands), (Kbd::Hidden, false));
    }

    #[test]
    fn comfortable_steps_down_hands_then_keyboard_as_space_shrinks() {
        let big = on(100, 40, Comfortable);
        assert_eq!((big.kbd, big.hands), (Kbd::Full, true));
        let mid = on(100, 26, Comfortable);
        assert_eq!((mid.kbd, mid.hands), (Kbd::Full, false));
        assert_eq!(on(100, 20, Comfortable).kbd, Kbd::Compact);
        assert_eq!(on(100, 13, Comfortable).kbd, Kbd::Hidden);
        assert_eq!((big.rows, big.spaced, big.text_width), (3, true, 70));
    }

    #[test]
    fn hands_switch_off_keeps_the_keyboard() {
        let p = plan(100, 40, Comfortable, true, false, false);
        assert_eq!((p.kbd, p.hands), (Kbd::Full, false));
    }

    #[test]
    fn narrow_window_hides_the_keyboard_and_shrinks_the_text() {
        let p = on(50, 40, Comfortable);
        assert_eq!((p.kbd, p.hands, p.text_width), (Kbd::Hidden, false, 50));
    }

    #[test]
    fn compact_density_is_text_only_and_tight() {
        let p = on(100, 40, Compact);
        assert_eq!(
            p,
            Plan {
                rows: 3,
                spaced: false,
                text_width: 60,
                kbd: Kbd::Hidden,
                hands: false,
                big: false
            }
        );
    }

    #[test]
    fn spacious_density_is_wider_with_more_rows() {
        let p = on(120, 45, Spacious);
        assert_eq!(
            (p.rows, p.spaced, p.text_width, p.kbd),
            (4, true, 84, Kbd::Full)
        );
        assert_eq!(on(70, 40, Spacious).text_width, 70);
    }

    #[test]
    fn tiny_area_still_fits_at_least_one_row() {
        for d in [Compact, Comfortable, Spacious] {
            let p = on(20, 4, d);
            let lines = p.rows * if p.spaced { 2 } else { 1 } + 1;
            assert!(p.rows >= 1 && lines <= 4, "{d:?} {p:?}");
            assert_eq!(p.kbd, Kbd::Hidden);
        }
    }

    #[test]
    fn large_density_uses_the_big_keyboard_and_wide_text() {
        let p = on(185, 45, Large);
        assert_eq!(
            (p.rows, p.spaced, p.text_width, p.kbd, p.hands),
            (4, true, 110, Kbd::Large, true)
        );
    }

    #[test]
    fn large_falls_back_to_the_normal_keyboard_when_short_or_narrow() {
        assert_eq!(on(185, 28, Large).kbd, Kbd::Full);
        let narrow = on(100, 45, Large);
        assert_eq!((narrow.kbd, narrow.text_width), (Kbd::Full, 100));
    }

    fn big(w: u16, h: u16, keyboard: bool) -> Plan {
        plan(w, h, Large, keyboard, false, true)
    }

    #[test]
    fn big_text_uses_the_width_with_three_rows() {
        let p = big(147, 38, false);
        assert_eq!((p.big, p.rows, p.text_width), (true, 3, 143));
        assert_eq!(
            big(200, 38, false).text_width,
            160,
            "at most 40 letters a line"
        );
    }

    #[test]
    fn big_text_drops_to_two_rows_with_the_keyboard() {
        let p = big(147, 38, true);
        assert_eq!((p.big, p.rows), (true, 2));
    }

    #[test]
    fn big_text_needs_room_for_24_letters_and_one_row() {
        assert!(!big(90, 38, false).big, "86 columns fit 21 letters");
        assert!(big(100, 38, false).big, "96 columns fit 24 letters");
        assert_eq!(big(147, 12, false).rows, 2);
        assert!(!big(147, 5, false).big);
    }

    #[test]
    fn big_text_switch_off_keeps_normal_text() {
        assert!(!plan(147, 38, Large, false, false, false).big);
    }
}
