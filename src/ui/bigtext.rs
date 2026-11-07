// large block digits for the results screen

// the full block glyph, built from its code point so source stays ascii
pub const BLOCK: &str = "\u{2588}";

// 3x5 digit font, '#' is a filled cell
const FONT: [[&str; 5]; 10] = [
    ["###", "# #", "# #", "# #", "###"],
    [" # ", "## ", " # ", " # ", "###"],
    ["###", "  #", "###", "#  ", "###"],
    ["###", "  #", "###", "  #", "###"],
    ["# #", "# #", "###", "  #", "  #"],
    ["###", "#  ", "###", "  #", "###"],
    ["###", "#  ", "###", "# #", "###"],
    ["###", "  #", "  #", "  #", "  #"],
    ["###", "# #", "###", "# #", "###"],
    ["###", "# #", "###", "  #", "###"],
];

// five text rows. each cell is two blocks wide so digits look square.
pub fn digits(n: u32) -> Vec<String> {
    let ds: Vec<usize> = n.to_string().bytes().map(|b| (b - b'0') as usize).collect();
    (0..5)
        .map(|row| {
            ds.iter()
                .map(|&d| {
                    FONT[d][row]
                        .chars()
                        .map(|c| {
                            if c == '#' {
                                BLOCK.repeat(2)
                            } else {
                                "  ".into()
                            }
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_five_rows_of_equal_width() {
        let rows = digits(42);
        assert_eq!(rows.len(), 5);
        let w = rows[0].chars().count();
        assert!(rows.iter().all(|r| r.chars().count() == w));
        // 2 digits x 6 cols + 2 col gap
        assert_eq!(w, 14);
    }

    #[test]
    fn draws_the_right_shape() {
        let b = BLOCK.repeat(2);
        assert_eq!(digits(7)[0], b.repeat(3));
        assert_eq!(digits(7)[4], format!("    {b}"));
        assert_eq!(digits(0)[2], format!("{b}  {b}"));
    }
}
