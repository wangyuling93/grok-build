//! Sakura theme — Ghostty Sakura palette.
//!
//! Source: `mbadolato/iTerm2-Color-Schemes` → `ghostty/Sakura`
//!
//! | Slot | Hex |
//! |------|-----|
//! | background | `#18131e` |
//! | foreground | `#dd7bdc` |
//! | cursor | `#ff65fd` |
//! | selection bg/fg | `#c05cbf` / `#24242e` |
//! | black | `#000000` |
//! | red / bright | `#d52370` / `#f41d99` |
//! | green / bright | `#41af1a` / `#22e529` |
//! | yellow / bright | `#bc7053` / `#f59574` |
//! | blue / bright | `#6964ab` / `#9892f1` |
//! | magenta / bright | `#c71fbf` / `#e90cdd` |
//! | cyan / bright | `#939393` / `#eeeeee` |
//! | white / bright | `#998eac` / `#cbb6ff` |
//!
//! Elevated UI surfaces are stepped off the Ghostty background for TUI
//! panels, hover, and scrollbars. Truecolor recommended.

use ratatui::style::{Color, Modifier};

use super::tokyonight::Theme;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

#[allow(dead_code)]
mod palette {
    use super::*;

    // Ghostty base
    pub const BG: Color = rgb(24, 19, 30); // #18131e
    pub const FG: Color = rgb(221, 123, 220); // #dd7bdc
    pub const CURSOR: Color = rgb(255, 101, 253); // #ff65fd
    pub const SEL_BG: Color = rgb(192, 92, 191); // #c05cbf
    pub const SEL_FG: Color = rgb(36, 36, 46); // #24242e

    // ANSI normal
    pub const BLACK: Color = rgb(0, 0, 0); // #000000
    pub const RED: Color = rgb(213, 35, 112); // #d52370
    pub const GREEN: Color = rgb(65, 175, 26); // #41af1a
    pub const YELLOW: Color = rgb(188, 112, 83); // #bc7053
    pub const BLUE: Color = rgb(105, 100, 171); // #6964ab
    pub const MAGENTA: Color = rgb(199, 31, 191); // #c71fbf
    pub const CYAN: Color = rgb(147, 147, 147); // #939393
    pub const WHITE: Color = rgb(153, 142, 172); // #998eac

    // ANSI bright
    pub const BRIGHT_RED: Color = rgb(244, 29, 153); // #f41d99
    pub const BRIGHT_GREEN: Color = rgb(34, 229, 41); // #22e529
    pub const BRIGHT_YELLOW: Color = rgb(245, 149, 116); // #f59574
    pub const BRIGHT_BLUE: Color = rgb(152, 146, 241); // #9892f1
    pub const BRIGHT_MAGENTA: Color = rgb(233, 12, 221); // #e90cdd
    pub const BRIGHT_CYAN: Color = rgb(238, 238, 238); // #eeeeee
    pub const BRIGHT_WHITE: Color = rgb(203, 182, 255); // #cbb6ff

    // Elevated ramp stepped off #18131e
    pub const SURFACE: Color = rgb(34, 28, 42); // #221C2A
    pub const ELEVATED: Color = rgb(46, 38, 58); // #2E263A
    pub const HIGHLIGHT_MED: Color = rgb(58, 48, 74); // #3A304A
    pub const HIGHLIGHT_HIGH: Color = rgb(78, 62, 98); // #4E3E62
}
use palette::*;

impl Theme {
    /// Sakura — Ghostty Sakura terminal palette.
    pub const fn sakura() -> Self {
        Self {
            bg_base: BG,
            bg_light: ELEVATED,
            bg_dark: SURFACE,
            bg_highlight: ELEVATED,
            bg_hover: HIGHLIGHT_MED,
            bg_terminal: BG,
            canvas: BG,

            accent_user: CURSOR,
            accent_assistant: BRIGHT_BLUE,
            accent_thinking: WHITE,
            accent_tool: CYAN,
            accent_system: BLUE,
            accent_error: BRIGHT_RED,
            accent_success: BRIGHT_GREEN,
            accent_running: BRIGHT_MAGENTA,
            accent_skill: MAGENTA,

            // Ghostty default fg is pink; bright white for long-form body text.
            text_primary: BRIGHT_WHITE,
            text_secondary: FG,

            gray_dim: CYAN,
            gray: WHITE,
            gray_bright: BRIGHT_CYAN,

            command: BRIGHT_YELLOW,
            path: YELLOW,
            running: BRIGHT_BLUE,
            warning: BRIGHT_YELLOW,

            fuzzy_accent: CURSOR,

            accent_plan: BRIGHT_YELLOW,
            accent_verify: BRIGHT_BLUE,
            accent_feedback: BRIGHT_GREEN,
            accent_remember: GREEN,

            selection_border: SEL_BG,
            hover_border: HIGHLIGHT_MED,
            prompt_border: HIGHLIGHT_MED,
            prompt_border_active: SEL_BG,

            accent_model: BRIGHT_BLUE,

            scrollbar_bg: SURFACE,
            scrollbar_fg: HIGHLIGHT_HIGH,

            diff_delete_bg: rgb(48, 18, 36),
            diff_delete_fg: BRIGHT_RED,
            diff_insert_bg: rgb(18, 40, 22),
            diff_insert_fg: BRIGHT_GREEN,
            diff_equal_fg: WHITE,
            diff_gutter_fg: CYAN,

            bg_visual: HIGHLIGHT_MED,

            paste_bg: SURFACE,
            paste_fg: FG,
            paste_dim: WHITE,

            md_heading_h1: BRIGHT_WHITE,
            md_heading_h1_mod: Modifier::BOLD,
            md_heading_h2: CURSOR,
            md_heading_h2_mod: Modifier::BOLD,
            md_heading_h3: BRIGHT_BLUE,
            md_heading_h3_mod: Modifier::BOLD,
            md_heading_h4: BRIGHT_MAGENTA,
            md_heading_h4_mod: Modifier::BOLD.union(Modifier::ITALIC),
            md_heading_h5: BRIGHT_YELLOW,
            md_heading_h5_mod: Modifier::BOLD,
            md_heading_h6: BLUE,
            md_heading_h6_mod: Modifier::BOLD,
            md_code: BRIGHT_BLUE,
            md_task_checked: BRIGHT_GREEN,
            md_task_unchecked: WHITE,
            md_muted: WHITE,
            md_code_bg: SURFACE,
            md_text: BRIGHT_WHITE,
            link_fg: CURSOR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn sakura_matches_ghostty_scheme() {
        let theme = Theme::sakura();
        assert!(theme.is_dark());
        assert_eq!(theme.bg_base, Color::Rgb(24, 19, 30)); // #18131e
        assert_eq!(theme.canvas, theme.bg_base);
        assert_eq!(theme.text_secondary, Color::Rgb(221, 123, 220)); // #dd7bdc
        assert_eq!(theme.accent_user, Color::Rgb(255, 101, 253)); // #ff65fd
        assert_eq!(theme.selection_border, Color::Rgb(192, 92, 191)); // #c05cbf
        assert_eq!(theme.accent_error, Color::Rgb(244, 29, 153)); // #f41d99
        assert_eq!(theme.accent_success, Color::Rgb(34, 229, 41)); // #22e529
    }
}
