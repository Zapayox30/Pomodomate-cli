use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Custom colors configuration for advanced styling
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomColors {
    pub tomato_red: Option<String>,
    pub nature_green: Option<String>,
    pub accent_purple: Option<String>,
    pub dark_bg: Option<String>,
    pub dark_base: Option<String>,
    pub border_dim: Option<String>,
    pub border_glow: Option<String>,
    pub muted_text: Option<String>,
    pub progress_bg: Option<String>,
    pub soft_white: Option<String>,
    pub warm_yellow: Option<String>,
}

/// Dynamic theme colors for UI customization
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub tomato_red: Color,
    pub nature_green: Color,
    pub accent_purple: Color,
    pub dark_bg: Color,
    pub dark_base: Color,
    pub border_dim: Color,
    pub border_glow: Color,
    pub muted_text: Color,
    pub progress_bg: Color,
    pub soft_white: Color,
    pub warm_yellow: Color,
}

fn parse_color(s: &str) -> Option<Color> {
    if let Ok(c) = Color::from_str(s) {
        return Some(c);
    }
    // Custom hex parser fallback (expects RRGGBB or #RRGGBB)
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}

impl ThemeColors {
    /// Retrieve ThemeColors based on theme name and custom overrides
    pub fn get(name: &str, custom: &Option<CustomColors>) -> Self {
        let mut base = match name.to_lowercase().as_str() {
            "nord" => Self::nord(),
            "dracula" => Self::dracula(),
            "gruvbox" => Self::gruvbox(),
            "monochrome" => Self::monochrome(),
            _ => Self::default_tomato(),
        };

        if let Some(custom) = custom {
            if let Some(ref c) = custom.tomato_red {
                if let Some(color) = parse_color(c) {
                    base.tomato_red = color;
                }
            }
            if let Some(ref c) = custom.nature_green {
                if let Some(color) = parse_color(c) {
                    base.nature_green = color;
                }
            }
            if let Some(ref c) = custom.accent_purple {
                if let Some(color) = parse_color(c) {
                    base.accent_purple = color;
                }
            }
            if let Some(ref c) = custom.dark_bg {
                if let Some(color) = parse_color(c) {
                    base.dark_bg = color;
                }
            }
            if let Some(ref c) = custom.dark_base {
                if let Some(color) = parse_color(c) {
                    base.dark_base = color;
                }
            }
            if let Some(ref c) = custom.border_dim {
                if let Some(color) = parse_color(c) {
                    base.border_dim = color;
                }
            }
            if let Some(ref c) = custom.border_glow {
                if let Some(color) = parse_color(c) {
                    base.border_glow = color;
                }
            }
            if let Some(ref c) = custom.muted_text {
                if let Some(color) = parse_color(c) {
                    base.muted_text = color;
                }
            }
            if let Some(ref c) = custom.progress_bg {
                if let Some(color) = parse_color(c) {
                    base.progress_bg = color;
                }
            }
            if let Some(ref c) = custom.soft_white {
                if let Some(color) = parse_color(c) {
                    base.soft_white = color;
                }
            }
            if let Some(ref c) = custom.warm_yellow {
                if let Some(color) = parse_color(c) {
                    base.warm_yellow = color;
                }
            }
        }

        base
    }

    fn default_tomato() -> Self {
        Self {
            tomato_red: Color::Rgb(192, 57, 43),
            nature_green: Color::Rgb(39, 174, 96),
            accent_purple: Color::Rgb(142, 68, 173),
            dark_bg: Color::Rgb(20, 30, 40),
            dark_base: Color::Rgb(28, 40, 51),
            border_dim: Color::Rgb(52, 73, 94),
            border_glow: Color::Rgb(80, 110, 140),
            muted_text: Color::Rgb(127, 140, 141),
            progress_bg: Color::Rgb(44, 62, 80),
            soft_white: Color::Rgb(236, 240, 241),
            warm_yellow: Color::Rgb(243, 156, 18),
        }
    }

    fn nord() -> Self {
        Self {
            tomato_red: Color::Rgb(191, 97, 106),
            nature_green: Color::Rgb(163, 190, 140),
            accent_purple: Color::Rgb(180, 142, 173),
            dark_bg: Color::Rgb(46, 52, 64),
            dark_base: Color::Rgb(59, 66, 82),
            border_dim: Color::Rgb(76, 86, 106),
            border_glow: Color::Rgb(136, 192, 208),
            muted_text: Color::Rgb(216, 222, 233),
            progress_bg: Color::Rgb(67, 76, 94),
            soft_white: Color::Rgb(236, 239, 244),
            warm_yellow: Color::Rgb(235, 203, 139),
        }
    }

    fn dracula() -> Self {
        Self {
            tomato_red: Color::Rgb(255, 85, 85),
            nature_green: Color::Rgb(80, 250, 123),
            accent_purple: Color::Rgb(189, 147, 249),
            dark_bg: Color::Rgb(40, 42, 54),
            dark_base: Color::Rgb(30, 31, 41),
            border_dim: Color::Rgb(68, 71, 90),
            border_glow: Color::Rgb(255, 121, 198),
            muted_text: Color::Rgb(98, 114, 164),
            progress_bg: Color::Rgb(68, 71, 90),
            soft_white: Color::Rgb(248, 248, 242),
            warm_yellow: Color::Rgb(241, 250, 140),
        }
    }

    fn gruvbox() -> Self {
        Self {
            tomato_red: Color::Rgb(204, 36, 29),
            nature_green: Color::Rgb(152, 151, 26),
            accent_purple: Color::Rgb(177, 98, 134),
            dark_bg: Color::Rgb(40, 40, 40),
            dark_base: Color::Rgb(29, 32, 33),
            border_dim: Color::Rgb(102, 92, 84),
            border_glow: Color::Rgb(215, 153, 33),
            muted_text: Color::Rgb(146, 131, 116),
            progress_bg: Color::Rgb(60, 56, 54),
            soft_white: Color::Rgb(235, 219, 178),
            warm_yellow: Color::Rgb(250, 189, 47),
        }
    }

    fn monochrome() -> Self {
        Self {
            tomato_red: Color::Rgb(255, 255, 255),
            nature_green: Color::Rgb(200, 200, 200),
            accent_purple: Color::Rgb(150, 150, 150),
            dark_bg: Color::Rgb(0, 0, 0),
            dark_base: Color::Rgb(20, 20, 20),
            border_dim: Color::Rgb(80, 80, 80),
            border_glow: Color::Rgb(255, 255, 255),
            muted_text: Color::Rgb(120, 120, 120),
            progress_bg: Color::Rgb(40, 40, 40),
            soft_white: Color::Rgb(255, 255, 255),
            warm_yellow: Color::Rgb(200, 200, 200),
        }
    }
}
