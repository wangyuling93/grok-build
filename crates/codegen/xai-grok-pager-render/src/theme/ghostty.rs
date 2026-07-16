//! Map Ghostty / iTerm2 terminal schemes into the pager [`Theme`].
//!
//! Terminal schemes only define bg/fg/cursor/selection + ANSI 16. We expand
//! that into the full TUI role map and synthesize elevated surfaces /
//! scrollbars so hover, panels, and chrome stay legible.

use ratatui::style::{Color, Modifier};

use super::ghostty_catalog::{GhosttyScheme, GHOSTTY_SCHEMES};
use super::osc11::classify_luminance;
use super::system_appearance::SystemAppearance;
use super::tokyonight::Theme;

const fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

/// Lighten or darken an RGB triple by `amount` (positive = toward white).
fn nudge(c: (u8, u8, u8), amount: i16) -> (u8, u8, u8) {
    let n = |v: u8| (v as i16 + amount).clamp(0, 255) as u8;
    (n(c.0), n(c.1), n(c.2))
}

fn lum(c: (u8, u8, u8)) -> u32 {
    // BT.709 approximate integer luminance (gamma-ignorant; fine for
    // relative body-text picks, not for dark/light polarity).
    (c.0 as u32 * 2126 + c.1 as u32 * 7152 + c.2 as u32 * 722) / 10000
}

/// Match [`Theme::is_dark`] polarity (sRGB-linear BT.709 via OSC11 classifier).
fn is_dark_bg(bg: (u8, u8, u8)) -> bool {
    classify_luminance(bg.0, bg.1, bg.2) == SystemAppearance::Dark
}

/// Ensure scrollbar thumb/track differ by ≥ `min_delta` summed-RGB in the
/// correct polarity (thumb lighter on dark, darker on light).
fn ensure_scrollbar_contrast(
    track: (u8, u8, u8),
    thumb: (u8, u8, u8),
    dark: bool,
    min_delta: i16,
) -> ((u8, u8, u8), (u8, u8, u8)) {
    let sum = |c: (u8, u8, u8)| c.0 as i32 + c.1 as i32 + c.2 as i32;
    let delta = sum(thumb) - sum(track);
    let ok = if dark {
        delta >= min_delta as i32
    } else {
        delta <= -(min_delta as i32)
    };
    if ok {
        return (track, thumb);
    }
    // Push thumb further away from canvas; keep track.
    let step = if dark { min_delta } else { -min_delta };
    let mut t = thumb;
    for _ in 0..8 {
        t = nudge(t, step);
        let d = sum(t) - sum(track);
        if dark && d >= min_delta as i32 {
            return (track, t);
        }
        if !dark && d <= -(min_delta as i32) {
            return (track, t);
        }
    }
    // Last resort: pure white/black thumb.
    if dark {
        (track, (255, 255, 255))
    } else {
        (track, (0, 0, 0))
    }
}

/// Whether the catalog entry at `index` is a dark canvas (cheap polarity).
#[must_use]
pub fn catalog_index_is_dark(index: u16) -> bool {
    GHOSTTY_SCHEMES
        .get(index as usize)
        .map(|s| is_dark_bg(s.background))
        .unwrap_or(true)
}

/// Build a full pager [`Theme`] from a Ghostty scheme.
#[must_use]
pub fn theme_from_ghostty(scheme: &GhosttyScheme) -> Theme {
    let bg = scheme.background;
    let fg = scheme.foreground;
    let cur = scheme.cursor;
    let sel_bg = scheme.selection_background;
    let sel_fg = scheme.selection_foreground;
    let p = scheme.palette;

    let dark = is_dark_bg(bg);
    // Elevated ramp: step away from canvas for panels / hover / scrollbar.
    let (surface, elevated, hover, chrome_hi) = if dark {
        (
            nudge(bg, 12),
            nudge(bg, 24),
            nudge(bg, 36),
            nudge(bg, 56),
        )
    } else {
        (
            nudge(bg, -10),
            nudge(bg, -20),
            nudge(bg, -32),
            nudge(bg, -48),
        )
    };

    // Scrollbar: thumb must contrast with track (Σrgb delta ≥ 30).
    let (scroll_track, scroll_thumb) = {
        let (track, thumb) = if dark {
            (surface, chrome_hi)
        } else {
            (elevated, chrome_hi)
        };
        ensure_scrollbar_contrast(track, thumb, dark, 30)
    };

    let black = p[0];
    let red = p[1];
    let green = p[2];
    let yellow = p[3];
    let blue = p[4];
    let magenta = p[5];
    let cyan = p[6];
    let white = p[7];
    let bright_black = p[8];
    let bright_red = p[9];
    let bright_green = p[10];
    let bright_yellow = p[11];
    let bright_blue = p[12];
    let bright_magenta = p[13];
    let bright_cyan = p[14];
    let bright_white = p[15];

    // Body text: prefer bright white / white when available for readability;
    // keep scheme fg as secondary (identity color, e.g. Sakura pink / Aurora amber).
    let (text_primary, text_secondary) = if dark {
        let body = if lum(bright_white) > lum(fg) {
            bright_white
        } else if lum(white) > lum(fg) {
            white
        } else {
            fg
        };
        (body, fg)
    } else {
        // Light canvas: scheme fg is usually the ink.
        (fg, bright_black)
    };

    let gray_dim = bright_black;
    let gray = if dark { white } else { bright_black };
    let gray_bright = if dark { bright_white } else { black };

    Theme {
        bg_base: rgb(bg),
        bg_light: rgb(elevated),
        bg_dark: rgb(surface),
        bg_highlight: rgb(elevated),
        bg_hover: rgb(hover),
        bg_terminal: rgb(bg),
        canvas: rgb(bg),

        accent_user: rgb(cur),
        accent_assistant: rgb(bright_magenta),
        accent_thinking: rgb(gray_dim),
        accent_tool: rgb(gray_dim),
        accent_system: rgb(blue),
        accent_error: rgb(bright_red),
        accent_success: rgb(bright_green),
        accent_running: rgb(bright_cyan),
        accent_skill: rgb(magenta),

        text_primary: rgb(text_primary),
        text_secondary: rgb(text_secondary),

        gray_dim: rgb(gray_dim),
        gray: rgb(gray),
        gray_bright: rgb(gray_bright),

        command: rgb(bright_yellow),
        path: rgb(yellow),
        running: rgb(cyan),
        warning: rgb(bright_yellow),

        fuzzy_accent: rgb(bright_blue),

        accent_plan: rgb(bright_yellow),
        accent_verify: rgb(bright_blue),
        accent_feedback: rgb(bright_cyan),
        accent_remember: rgb(green),

        selection_border: rgb(sel_fg),
        hover_border: rgb(hover),
        prompt_border: rgb(hover),
        prompt_border_active: rgb(sel_fg),

        accent_model: rgb(cyan),

        scrollbar_bg: rgb(scroll_track),
        scrollbar_fg: rgb(scroll_thumb),

        diff_delete_bg: rgb(if dark {
            nudge(red, -80)
        } else {
            nudge(red, 80)
        }),
        diff_delete_fg: rgb(bright_red),
        diff_insert_bg: rgb(if dark {
            nudge(green, -80)
        } else {
            nudge(green, 80)
        }),
        diff_insert_fg: rgb(bright_green),
        diff_equal_fg: rgb(gray_dim),
        diff_gutter_fg: rgb(gray_dim),

        bg_visual: rgb(sel_bg),

        paste_bg: rgb(surface),
        paste_fg: rgb(text_secondary),
        paste_dim: rgb(gray_dim),

        md_heading_h1: rgb(text_primary),
        md_heading_h1_mod: Modifier::BOLD,
        md_heading_h2: rgb(bright_cyan),
        md_heading_h2_mod: Modifier::BOLD,
        md_heading_h3: rgb(bright_magenta),
        md_heading_h3_mod: Modifier::BOLD,
        md_heading_h4: rgb(bright_green),
        md_heading_h4_mod: Modifier::BOLD.union(Modifier::ITALIC),
        md_heading_h5: rgb(bright_yellow),
        md_heading_h5_mod: Modifier::BOLD,
        md_heading_h6: rgb(blue),
        md_heading_h6_mod: Modifier::BOLD,
        md_code: rgb(cyan),
        md_task_checked: rgb(bright_green),
        md_task_unchecked: rgb(gray),
        md_muted: rgb(gray_dim),
        md_code_bg: rgb(surface),
        md_text: rgb(text_primary),
        link_fg: rgb(if dark { cur } else { blue }),
    }
}

/// Theme for Ghostty catalog index, or [`Theme::groknight`] if out of range.
#[must_use]
pub fn theme_from_ghostty_index(index: u16) -> Theme {
    GHOSTTY_SCHEMES
        .get(index as usize)
        .map(theme_from_ghostty)
        .unwrap_or_else(Theme::groknight)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ghostty_catalog::{scheme_by_slug, GHOSTTY_SCHEMES};
    use ratatui::style::Color;

    #[test]
    fn catalog_is_non_empty_and_indexed() {
        assert!(GHOSTTY_SCHEMES.len() > 500);
        assert!(GHOSTTY_SCHEMES.len() <= u16::MAX as usize);
    }

    #[test]
    fn sakura_and_aurora_are_first_class_not_catalog() {
        // Hand-mapped first-class themes; catalog generator excludes these slugs.
        assert!(scheme_by_slug("sakura").is_none());
        assert!(scheme_by_slug("aurora").is_none());
        let sakura = Theme::sakura();
        assert!(sakura.is_dark());
        assert_eq!(sakura.bg_base, Color::Rgb(24, 19, 30));
        let aurora = Theme::aurora();
        assert!(aurora.is_dark());
        assert_eq!(aurora.bg_base, Color::Rgb(35, 38, 46));
    }

    #[test]
    fn every_scheme_builds_and_has_scrollbar_contrast() {
        for (i, scheme) in GHOSTTY_SCHEMES.iter().enumerate() {
            let t = theme_from_ghostty(scheme);
            assert_eq!(t.canvas, t.bg_base, "{} canvas", scheme.display);
            // Scrollbar polarity check (same rule as theme tests).
            let sum = |c: Color| match c {
                Color::Rgb(r, g, b) => r as i32 + g as i32 + b as i32,
                _ => panic!("{} non-rgb scrollbar", scheme.display),
            };
            let delta = sum(t.scrollbar_fg) - sum(t.scrollbar_bg);
            if t.is_dark() {
                assert!(
                    delta >= 30,
                    "{} dark scrollbar Δ{delta} (idx {i})",
                    scheme.display
                );
            } else {
                assert!(
                    delta <= -30,
                    "{} light scrollbar Δ{delta} (idx {i})",
                    scheme.display
                );
            }
        }
    }
}
