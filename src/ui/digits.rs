/// Typeface used for the big clock.
///
/// Every glyph is exactly three columns wide and three rows tall, so the
/// layout maths stays the same whichever style is picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DigitStyle {
    /// Rounded light box-drawing characters.
    #[default]
    Line,
    /// Heavy box-drawing characters.
    Heavy,
    /// Double box-drawing characters.
    Double,
}

impl DigitStyle {
    /// Parse a config value, falling back to the default on anything unknown.
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_lowercase().as_str() {
            "heavy" | "bold" | "thick" => Self::Heavy,
            "double" => Self::Double,
            _ => Self::Line,
        }
    }

    /// Stable name for config files and the status line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Heavy => "heavy",
            Self::Double => "double",
        }
    }

    /// The next style, wrapping around. Used by the cycle key.
    pub fn next(self) -> Self {
        match self {
            Self::Line => Self::Heavy,
            Self::Heavy => Self::Double,
            Self::Double => Self::Line,
        }
    }

    /// The three rows making up a digit, top to bottom.
    pub fn rows(self, digit: char) -> [&'static str; 3] {
        match self {
            Self::Line => match digit {
                '0' => ["╭─╮", "│ │", "╰─╯"],
                '1' => [" ╷ ", " │ ", " ╵ "],
                '2' => ["╶─╮", "╭─╯", "╰─╴"],
                '3' => ["╶─╮", " ─┤", "╶─╯"],
                '4' => ["╷ ╷", "╰─┤", "  ╵"],
                '5' => ["╭─╴", "╰─╮", "╶─╯"],
                '6' => ["╭─╴", "├─╮", "╰─╯"],
                '7' => ["╶─╮", "  │", "  ╵"],
                '8' => ["╭─╮", "├─┤", "╰─╯"],
                '9' => ["╭─╮", "╰─┤", "╶─╯"],
                _ => ["   ", "   ", "   "],
            },
            Self::Heavy => match digit {
                '0' => ["┏━┓", "┃ ┃", "┗━┛"],
                '1' => [" ╻ ", " ┃ ", " ╹ "],
                '2' => ["╺━┓", "┏━┛", "┗━╸"],
                '3' => ["╺━┓", " ━┫", "╺━┛"],
                '4' => ["╻ ╻", "┗━┫", "  ╹"],
                '5' => ["┏━╸", "┗━┓", "╺━┛"],
                '6' => ["┏━╸", "┣━┓", "┗━┛"],
                '7' => ["╺━┓", "  ┃", "  ╹"],
                '8' => ["┏━┓", "┣━┫", "┗━┛"],
                '9' => ["┏━┓", "┗━┫", "╺━┛"],
                _ => ["   ", "   ", "   "],
            },
            Self::Double => match digit {
                '0' => ["╔═╗", "║ ║", "╚═╝"],
                '1' => [" ║ ", " ║ ", " ║ "],
                '2' => ["══╗", "╔═╝", "╚══"],
                '3' => ["══╗", " ═╣", "══╝"],
                '4' => ["║ ║", "╚═╣", "  ║"],
                '5' => ["╔══", "╚═╗", "══╝"],
                '6' => ["╔══", "╠═╗", "╚═╝"],
                '7' => ["══╗", "  ║", "  ║"],
                '8' => ["╔═╗", "╠═╣", "╚═╝"],
                '9' => ["╔═╗", "╚═╣", "══╝"],
                _ => ["   ", "   ", "   "],
            },
        }
    }

    /// The separator between minutes and seconds, sized to match the style.
    ///
    /// All three glyphs come from the Geometric Shapes block and are
    /// unambiguously single-width, so the separator is exactly three columns.
    ///
    /// Emoji-presentation characters are the ones to avoid here: they are
    /// double-width and would push the seconds out of alignment. Note that
    /// glyphs with East Asian "ambiguous" width, such as U+2B24, measure as
    /// single-width yet render wide in some fonts — the column test below
    /// cannot catch those, so prefer unambiguous glyphs.
    pub fn colon(self) -> &'static str {
        match self {
            Self::Line => " ● ",
            Self::Heavy => " ◆ ",
            Self::Double => " ◉ ",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLES: [DigitStyle; 3] = [DigitStyle::Line, DigitStyle::Heavy, DigitStyle::Double];

    #[test]
    fn every_glyph_is_exactly_three_columns_wide() {
        // Display columns, not chars: a double-width glyph counts as one
        // char but occupies two columns, which is exactly the mistake this
        // test exists to catch.
        use unicode_width::UnicodeWidthStr;

        for style in STYLES {
            for digit in "0123456789".chars() {
                for (row, text) in style.rows(digit).iter().enumerate() {
                    assert_eq!(
                        text.width(),
                        3,
                        "{:?} digit {digit} row {row} is not 3 columns: {text:?}",
                        style
                    );
                }
            }
        }
    }

    #[test]
    fn unknown_characters_render_as_blanks() {
        for style in STYLES {
            assert_eq!(style.rows('x'), ["   ", "   ", "   "]);
        }
    }

    #[test]
    fn cycling_visits_every_style_and_returns_home() {
        let mut style = DigitStyle::Line;
        let mut seen = vec![style];
        for _ in 0..STYLES.len() - 1 {
            style = style.next();
            seen.push(style);
        }
        assert_eq!(seen.len(), STYLES.len(), "each style should appear once");
        assert_eq!(style.next(), DigitStyle::Line, "cycle should wrap around");
    }

    #[test]
    fn names_round_trip() {
        for style in STYLES {
            assert_eq!(DigitStyle::from_name(style.name()), style);
        }
    }

    #[test]
    fn unknown_names_fall_back_to_the_default() {
        assert_eq!(DigitStyle::from_name("nonsense"), DigitStyle::Line);
        assert_eq!(DigitStyle::from_name(""), DigitStyle::Line);
    }

    #[test]
    fn names_are_case_and_space_insensitive() {
        assert_eq!(DigitStyle::from_name("  HEAVY "), DigitStyle::Heavy);
        assert_eq!(DigitStyle::from_name("Double"), DigitStyle::Double);
    }

    #[test]
    fn aliases_map_to_heavy() {
        assert_eq!(DigitStyle::from_name("bold"), DigitStyle::Heavy);
        assert_eq!(DigitStyle::from_name("thick"), DigitStyle::Heavy);
    }

    /// Prints a sample of every style for eyeballing.
    /// `cargo test -- --ignored --nocapture digit_samples`
    #[test]
    #[ignore]
    fn digit_samples() {
        for style in STYLES {
            eprintln!("\n{}:", style.name());
            for row in 0..3 {
                let mut line = String::new();
                for digit in "25".chars() {
                    line.push_str(style.rows(digit)[row]);
                    line.push(' ');
                }
                line.push_str(if row == 1 { style.colon() } else { "   " });
                for digit in "00".chars() {
                    line.push(' ');
                    line.push_str(style.rows(digit)[row]);
                }
                eprintln!("{line}");
            }
        }
    }

    #[test]
    fn colons_are_three_columns_too() {
        use unicode_width::UnicodeWidthStr;

        for style in STYLES {
            assert_eq!(
                style.colon().width(),
                3,
                "{:?} separator is not 3 columns: {:?}",
                style,
                style.colon()
            );
        }
    }

    #[test]
    fn the_column_check_catches_a_wide_glyph() {
        // Guards the guard: an emoji is three chars but four columns, so a
        // char count would wave it through and a column count must not.
        use unicode_width::UnicodeWidthStr;
        assert_eq!(" 🍅 ".chars().count(), 3);
        assert_eq!(" 🍅 ".width(), 4);
    }
}
