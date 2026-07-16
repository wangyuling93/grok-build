//! Aurora theme — Ghostty Aurora palette.
//!
//! Source: `mbadolato/iTerm2-Color-Schemes` → `ghostty/Aurora`
//!
//! | Slot | Hex |
//! |------|-----|
//! | background | `#23262e` |
//! | foreground | `#ffca28` |
//! | cursor | `#ee5d43` |
//! | selection bg/fg | `#292e38` / `#00e8c6` |
//! | black / bright | `#23262e` / `#4f545e` |
//! | red / bright | `#f0266f` / `#f92672` |
//! | green | `#8fd46d` |
//! | yellow | `#ffe66d` |
//! | blue / bright | `#102ee4` / `#03d6b8` |
//! | magenta / bright | `#ee5d43` / `#ee5d43` |
//! | cyan / bright | `#03d6b8` / `#03d6b8` |
//! | white / bright | `#c74ded` / `#c74ded` |
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
    pub const BG: Color = rgb(35, 38, 46); // #23262e
    pub const FG: Color = rgb(255, 202, 40); // #ffca28
    pub const CURSOR: Color = rgb(238, 93, 67); // #ee5d43
    pub const SEL_BG: Color = rgb(41, 46, 56); // #292e38
    pub const SEL_FG: Color = rgb(0, 232, 198); // #00e8c6
    pub const CURSOR_TEXT: Color = rgb(255, 210, 156); // #ffd29c

    // ANSI normal
    pub const BLACK: Color = rgb(35, 38, 46); // #23262e
    pub const RED: Color = rgb(240, 38, 111); // #f0266f
    pub const GREEN: Color = rgb(143, 212, 109); // #8fd46d
    pub const YELLOW: Color = rgb(255, 230, 109); // #ffe66d
    pub const BLUE: Color = rgb(16, 46, 228); // #102ee4
    pub const MAGENTA: Color = rgb(238, 93, 67); // #ee5d43
    pub const CYAN: Color = rgb(3, 214, 184); // #03d6b8
    pub const WHITE: Color = rgb(199, 77, 237); // #c74ded

    // ANSI bright (Ghostty Aurora reuses several normal slots)
    pub const BRIGHT_BLACK: Color = rgb(79, 84, 94); // #4f545e
    pub const BRIGHT_RED: Color = rgb(249, 38, 114); // #f92672
    pub const BRIGHT_GREEN: Color = rgb(143, 212, 109); // #8fd46d
    pub const BRIGHT_YELLOW: Color = rgb(255, 230, 109); // #ffe66d
    pub const BRIGHT_BLUE: Color = rgb(3, 214, 184); // #03d6b8
    pub const BRIGHT_MAGENTA: Color = rgb(238, 93, 67); // #ee5d43
    pub const BRIGHT_CYAN: Color = rgb(3, 214, 184); // #03d6b8
    pub const BRIGHT_WHITE: Color = rgb(199, 77, 237); // #c74ded

    // Elevated ramp stepped off #23262e
    pub const SURFACE: Color = rgb(41, 46, 56); // #292e38 (selection bg)
    pub const ELEVATED: Color = rgb(52, 58, 70); // #343A46
    pub const HIGHLIGHT_MED: Color = rgb(64, 72, 88); // #404858
    pub const HIGHLIGHT_HIGH: Color = rgb(88, 98, 118); // #586276
}
use palette::*;

impl Theme {
    /// Aurora — Ghostty Aurora terminal palette.
    pub const fn aurora() -> Self {
        Self {
            bg_base: BG,
            bg_light: ELEVATED,
            bg_dark: SURFACE,
            bg_highlight: ELEVATED,
            bg_hover: HIGHLIGHT_MED,
            bg_terminal: BG,
            canvas: BG,

            // Amber/orange identity from Ghostty fg + cursor.
            accent_user: CURSOR,
            accent_assistant: WHITE,
            accent_thinking: BRIGHT_BLACK,
            accent_tool: BRIGHT_BLACK,
            accent_system: BLUE,
            accent_error: BRIGHT_RED,
            accent_success: GREEN,
            accent_running: CYAN,
            accent_skill: BRIGHT_BLUE,

            // Ghostty default fg is amber gold — use it as secondary chrome.
            // Body text is a soft warm off-white so long scrollback stays
            // readable (same tradeoff Sakura makes with bright lavender).
            text_primary: CURSOR_TEXT, // #ffd29c warm cream
            text_secondary: FG,        // #ffca28 Ghostty amber

            // Distinct gray ramp (Ghostty reuses bright-black; split for TUI).
            gray_dim: rgb(58, 62, 72),  // dimmer than bright-black
            gray: BRIGHT_BLACK,        // #4f545e
            gray_bright: CURSOR_TEXT,  // warm cream

            command: YELLOW,
            path: MAGENTA,
            running: CYAN,
            warning: YELLOW,

            fuzzy_accent: CYAN,

            accent_plan: YELLOW,
            accent_verify: WHITE,
            accent_feedback: CYAN,
            accent_remember: GREEN,

            selection_border: SEL_FG,
            hover_border: HIGHLIGHT_MED,
            prompt_border: HIGHLIGHT_MED,
            prompt_border_active: SEL_FG,

            accent_model: CYAN,

            // Thumb lighter than track on dark canvas.
            scrollbar_bg: SURFACE,
            scrollbar_fg: HIGHLIGHT_HIGH,

            diff_delete_bg: rgb(56, 24, 36),
            diff_delete_fg: BRIGHT_RED,
            diff_insert_bg: rgb(28, 48, 32),
            diff_insert_fg: GREEN,
            diff_equal_fg: BRIGHT_BLACK,
            diff_gutter_fg: BRIGHT_BLACK,

            bg_visual: HIGHLIGHT_MED,

            paste_bg: SURFACE,
            paste_fg: CURSOR_TEXT,
            paste_dim: BRIGHT_BLACK,

            md_heading_h1: CURSOR_TEXT,
            md_heading_h1_mod: Modifier::BOLD,
            md_heading_h2: CYAN,
            md_heading_h2_mod: Modifier::BOLD,
            md_heading_h3: WHITE,
            md_heading_h3_mod: Modifier::BOLD,
            md_heading_h4: GREEN,
            md_heading_h4_mod: Modifier::BOLD.union(Modifier::ITALIC),
            md_heading_h5: YELLOW,
            md_heading_h5_mod: Modifier::BOLD,
            md_heading_h6: CURSOR,
            md_heading_h6_mod: Modifier::BOLD,
            md_code: CYAN,
            md_task_checked: GREEN,
            md_task_unchecked: BRIGHT_BLACK,
            md_muted: BRIGHT_BLACK,
            md_code_bg: SURFACE,
            md_text: CURSOR_TEXT,
            link_fg: SEL_FG,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn aurora_matches_ghostty_scheme() {
        let theme = Theme::aurora();
        assert!(theme.is_dark(), "Ghostty Aurora is a dark scheme");
        assert_eq!(theme.bg_base, Color::Rgb(35, 38, 46)); // #23262e
        assert_eq!(theme.canvas, theme.bg_base);
        // Body cream; Ghostty amber kept as secondary identity.
        assert_eq!(theme.text_primary, Color::Rgb(255, 210, 156)); // #ffd29c
        assert_eq!(theme.text_secondary, Color::Rgb(255, 202, 40)); // #ffca28
        assert_ne!(theme.gray_dim, theme.gray, "gray ramp must be distinct");
        assert_eq!(theme.accent_user, Color::Rgb(238, 93, 67)); // #ee5d43
        assert_eq!(theme.running, Color::Rgb(3, 214, 184)); // #03d6b8
        assert_eq!(theme.accent_error, Color::Rgb(249, 38, 114)); // #f92672
        assert_eq!(theme.accent_success, Color::Rgb(143, 212, 109)); // #8fd46d
        assert_eq!(theme.accent_assistant, Color::Rgb(199, 77, 237)); // #c74ded
    }
}
