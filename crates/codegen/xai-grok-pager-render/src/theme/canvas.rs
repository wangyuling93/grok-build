//! Transparent body canvas + solid endpoints for blend / invert math.
//!
//! Paint fills (`bg_base`, elevated passive surfaces) may be [`Color::Reset`]
//! under `[ui].transparent_background`. Blend and inverse-chip math still need
//! a solid design endpoint — that is [`Theme::canvas`] / [`Theme::design_canvas`].
//!
//! ## Public surface on [`Theme`]
//!
//! | Method | Use when |
//! |--------|----------|
//! | [`Theme::design_canvas`] | Dim/fade toward the **body** canvas |
//! | [`Theme::solid_paint`] | Resolve a possibly-`Reset` paint for area fades |
//! | [`Theme::blend`] | Blend a color toward a paint surface |
//! | [`Theme::blend_canvas`] | Body-canvas blend (no paint arg) |
//! | [`Theme::invert_ink`] | Inverse-chip ink when paint may be elevated **or** `Reset` |
//! | [`Theme::invert_canvas`] | Body inverse-chip ink (no paint arg) |
//!
//! Body dim: `dim_area(buf, area, theme.design_canvas(), factor)`.
//! Body accent: `theme.blend_canvas(accent, opacity)`.
//! Body inverse chip: `theme.invert_canvas()`.
//! Local elevated: `theme.blend(local_paint, color, opacity)` or
//! `blend_area(..., theme.solid_paint(local_paint), ...)`.
//! Elevated inverse chip: `theme.invert_ink(paint)`.

use ratatui::style::Color;

use super::Theme;

impl Theme {
    /// Apply paint-time mode flags from the theme cache.
    ///
    /// Today this is only `[ui].transparent_background` via
    /// [`Self::transparent_elevated`]. Called at the end of [`Theme::current`]
    /// so tests can exercise the same gate without reconstructing the full
    /// quantize / ANSI pipeline.
    #[must_use]
    pub(crate) fn apply_paint_mode(self) -> Self {
        if super::cache::load_transparent_background() {
            self.transparent_elevated()
        } else {
            self
        }
    }

    /// Force body + passive elevated surfaces to [`Color::Reset`] so the host
    /// terminal profile shows through. Text, accents, borders, semantic diff
    /// bands, and **interaction** surfaces (`bg_hover`, `bg_visual`,
    /// `bg_dark`, `bg_highlight`) stay solid so hover/selection still read.
    ///
    /// Leaves [`Self::canvas`] alone — that is the solid design endpoint for
    /// blend / fade / dim math (or [`Color::Reset`] for terminal-native).
    ///
    /// Applied by [`Theme::apply_paint_mode`] when transparent mode is on.
    #[must_use]
    pub fn transparent_elevated(self) -> Self {
        Self {
            bg_base: Color::Reset,
            bg_light: Color::Reset,
            bg_terminal: Color::Reset,
            md_code_bg: Color::Reset,
            paste_bg: Color::Reset,
            scrollbar_bg: Color::Reset,
            // Interaction / selection bands stay solid for legibility.
            // bg_dark, bg_highlight, bg_hover, bg_visual left as-is via ..self.
            // canvas left as-is — design endpoint for math.
            ..self
        }
    }

    /// Solid design canvas for blend / fade / dim math.
    ///
    /// Opaque solid themes: same as paint `bg_base`. Transparent: the solid
    /// design RGB kept in [`Self::canvas`]. Terminal-native / minimal:
    /// [`Color::Reset`] (so `blend_color` fails soft and accents stay undimmed).
    ///
    /// Body-only; for a possibly-elevated paint color use [`Self::solid_paint`].
    #[must_use]
    pub fn design_canvas(&self) -> Color {
        self.canvas
    }

    /// Resolve a paint color to a solid endpoint for area fades / blend math.
    ///
    /// When `paint` is [`Color::Reset`] (transparent canvas or transparent
    /// elevated slot), returns [`Self::design_canvas`]. Otherwise returns
    /// `paint` unchanged.
    #[must_use]
    pub fn solid_paint(&self, paint: Color) -> Color {
        match paint {
            Color::Reset => self.design_canvas(),
            other => other,
        }
    }

    /// Blend `color` toward `paint` at `opacity` (0 = paint base, 1 = color).
    ///
    /// When `paint` is [`Color::Reset`], uses the design canvas via
    /// [`Self::solid_paint`]. Returns `None` when the resolved base is non-RGB
    /// (terminal-native) so callers can fall back to the unblended color.
    ///
    /// Body-canvas blends: prefer [`Self::blend_canvas`].
    /// Local-surface blends (chrome, elevated panel): pass that surface as
    /// `paint`.
    #[must_use]
    pub fn blend(&self, paint: Color, color: Color, opacity: f32) -> Option<Color> {
        crate::render::color::blend_color(self.solid_paint(paint), color, opacity)
    }

    /// Blend `color` toward the body design canvas.
    ///
    /// Prefer this for body accents, waves, and dims so call sites do not
    /// re-pass paint. Local elevated surfaces still use [`Self::blend`].
    ///
    /// Goes straight to [`Self::design_canvas`] (not paint `bg_base`) so
    /// transparent mode never routes through a `Reset` → solid resolve step.
    #[must_use]
    pub fn blend_canvas(&self, color: Color, opacity: f32) -> Option<Color> {
        crate::render::color::blend_color(self.design_canvas(), color, opacity)
    }

    /// Foreground for inverse-video chips (cursor block, selected badge).
    ///
    /// | `paint` | Mode | Result |
    /// |---------|------|--------|
    /// | solid | any | `paint` (surface is the ink) |
    /// | [`Color::Reset`] | solid design canvas | Black/White from design polarity |
    /// | [`Color::Reset`] | terminal-native (`canvas` is Reset) | [`Color::Reset`] — reverse video |
    ///
    /// Transparent and terminal-native both paint `Reset`, but only solid
    /// themes keep a design RGB in [`Self::canvas`]. Polarity ink without a
    /// design canvas would hardcode Black on light host profiles (minimal
    /// mode) — wrong. Leave `Reset` so selection / chips fall back to
    /// `Modifier::REVERSED`.
    ///
    /// Body chips: prefer [`Self::invert_canvas`].
    #[must_use]
    pub fn invert_ink(&self, paint: Color) -> Color {
        match paint {
            Color::Reset => match self.canvas {
                // Terminal-native: no design canvas → reverse-video path.
                Color::Reset => Color::Reset,
                // Solid design canvas (opaque or transparent solid theme).
                _ => {
                    if self.is_dark() {
                        Color::Black
                    } else {
                        Color::White
                    }
                }
            },
            other => other,
        }
    }

    /// Inverse-chip ink against the body canvas.
    ///
    /// Prefer this for body selection / cursor chips so call sites do not
    /// re-pass `bg_base`. Elevated chips still use [`Self::invert_ink`].
    #[must_use]
    pub fn invert_canvas(&self) -> Color {
        self.invert_ink(self.bg_base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hold the shared theme cache lock and reset globals for a hermetic body.
    fn with_theme_cache(f: impl FnOnce()) {
        let _guard = crate::theme::cache::test_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::theme::cache::reset_for_test();
        f();
        crate::theme::cache::reset_for_test();
    }

    #[test]
    fn transparent_elevated_clears_body_and_elevated_surfaces() {
        let solid = Theme::groknight();
        assert!(matches!(solid.bg_base, Color::Rgb(_, _, _)));
        assert!(matches!(solid.bg_light, Color::Rgb(_, _, _)));
        let see = solid.transparent_elevated();
        // Body + passive elevated → Reset.
        for (name, color) in [
            ("bg_base", see.bg_base),
            ("bg_light", see.bg_light),
            ("bg_terminal", see.bg_terminal),
            ("md_code_bg", see.md_code_bg),
            ("paste_bg", see.paste_bg),
            ("scrollbar_bg", see.scrollbar_bg),
        ] {
            assert_eq!(color, Color::Reset, "{name} must be transparent");
        }
        // Interaction / selection bands stay solid for legibility.
        assert_eq!(see.bg_dark, solid.bg_dark);
        assert_eq!(see.bg_highlight, solid.bg_highlight);
        assert_eq!(see.bg_hover, solid.bg_hover);
        assert_eq!(see.bg_visual, solid.bg_visual);
        // Accents / text stay solid.
        assert_eq!(see.text_primary, solid.text_primary);
        assert_eq!(see.accent_user, solid.accent_user);
        // Design canvas retained for blend math (not pure black, not Reset).
        assert_eq!(see.design_canvas(), solid.bg_base);
        assert_eq!(see.canvas, solid.canvas);
        assert_eq!(see.invert_canvas(), Color::Black);
        assert_eq!(see.invert_ink(Color::Reset), Color::Black);
        assert_eq!(see.solid_paint(Color::Reset), see.design_canvas());
        assert_eq!(see.solid_paint(solid.bg_dark), solid.bg_dark);
        assert!(see.is_dark());
    }

    #[test]
    fn design_canvas_and_ink_pass_through_solid_bg_base() {
        let solid = Theme::groknight();
        assert_eq!(solid.design_canvas(), solid.bg_base);
        assert_eq!(solid.canvas, solid.bg_base);
        assert_eq!(solid.invert_canvas(), solid.bg_base);
        assert_eq!(solid.solid_paint(solid.bg_base), solid.bg_base);
        assert_eq!(
            solid.blend_canvas(solid.accent_user, 1.0),
            solid.blend(solid.bg_base, solid.accent_user, 1.0)
        );
        assert_eq!(solid.invert_canvas(), solid.invert_ink(solid.bg_base));

        let light = Theme::grokday().transparent_elevated();
        assert_eq!(light.design_canvas(), Theme::grokday().bg_base);
        assert_eq!(light.invert_canvas(), Color::White);
        assert!(!light.is_dark());
        assert_eq!(light.solid_paint(light.bg_base), light.design_canvas());

        // Terminal-native: no design canvas — blends fail soft; invert stays
        // Reset so selection chips use reverse video (not hardcoded Black).
        let term = Theme::terminal_default();
        assert_eq!(term.design_canvas(), Color::Reset);
        assert_eq!(term.canvas, Color::Reset);
        assert!(term.blend_canvas(Color::Rgb(255, 0, 0), 0.5).is_none());
        assert_eq!(term.invert_canvas(), Color::Reset);
        assert_eq!(term.invert_ink(Color::Reset), Color::Reset);

        // Local-surface blend falls back to design canvas when paint is Reset.
        let see = Theme::groknight().transparent_elevated();
        assert_eq!(
            see.blend(Color::Reset, see.accent_user, 1.0),
            see.blend_canvas(see.accent_user, 1.0)
        );
        assert_eq!(
            see.blend(see.bg_dark, see.accent_user, 1.0)
                .map(|c| matches!(c, Color::Rgb(_, _, _))),
            Some(true)
        );
    }

    #[test]
    fn invert_ink_distinguishes_transparent_from_terminal_native() {
        // Transparent solid theme: Reset paint → polarity ink from solid canvas.
        let dark = Theme::groknight().transparent_elevated();
        assert_ne!(dark.canvas, Color::Reset);
        assert_eq!(dark.bg_base, Color::Reset);
        assert_eq!(dark.invert_canvas(), Color::Black);
        assert_eq!(dark.invert_ink(Color::Reset), Color::Black);
        assert_eq!(dark.invert_ink(dark.bg_dark), dark.bg_dark);

        let light = Theme::grokday().transparent_elevated();
        assert_eq!(light.invert_canvas(), Color::White);
        assert_eq!(light.invert_ink(Color::Reset), Color::White);

        // Terminal-native: Reset canvas → Reset ink (not Black).
        let term = Theme::terminal_default();
        assert_eq!(term.canvas, Color::Reset);
        assert_eq!(term.invert_canvas(), Color::Reset);
        assert_eq!(term.invert_ink(Color::Reset), Color::Reset);

        // Opaque solid: body ink is the solid paint itself.
        let solid = Theme::groknight();
        assert_eq!(solid.invert_canvas(), solid.bg_base);
        // Explicit Reset paint with a solid design canvas → polarity ink.
        assert_eq!(solid.invert_ink(Color::Reset), Color::Black);
    }

    #[test]
    fn transparent_elevated_is_idempotent() {
        let once = Theme::groknight().transparent_elevated();
        let twice = once.transparent_elevated();
        assert_eq!(twice.design_canvas(), once.design_canvas());
        assert_eq!(twice.canvas, once.canvas);
        assert_eq!(twice.bg_base, Color::Reset);
    }

    #[test]
    fn apply_paint_mode_honors_transparent_cache_flag() {
        // Same gate [`Theme::current`] ends with (TrueColor so canvas holds RGB).
        with_theme_cache(|| {
            let run = |transparent: bool| {
                crate::theme::cache::set_transparent_background(transparent);
                Theme::groknight()
                    .quantized(crate::theme::color_support::ColorLevel::TrueColor)
                    .apply_paint_mode()
            };

            let solid = run(false);
            assert!(
                !matches!(solid.bg_base, Color::Reset),
                "opaque pipeline must paint a solid body, got {:?}",
                solid.bg_base
            );
            assert_eq!(solid.canvas, solid.bg_base);

            let see = run(true);
            assert_eq!(see.bg_base, Color::Reset);
            assert_eq!(see.bg_light, Color::Reset);
            assert_eq!(see.canvas, solid.canvas);
            assert_eq!(see.design_canvas(), solid.bg_base);
            assert_eq!(see.bg_dark, solid.bg_dark);
            assert_eq!(see.bg_hover, solid.bg_hover);
            assert!(see.blend_canvas(see.accent_user, 0.5).is_some());
            assert_eq!(see.invert_canvas(), Color::Black);

            let restored = run(false);
            assert_eq!(restored.bg_base, solid.bg_base);
            assert_eq!(restored.canvas, solid.canvas);
        });
    }

    #[test]
    fn theme_current_applies_paint_mode() {
        // Live `Theme::current` path. Always asserts body paint + flag wiring.
        // When the process color level still has color (not `NO_COLOR` /
        // ColorLevel::None), also asserts the design canvas stays solid.
        with_theme_cache(|| {
            crate::theme::cache::set(crate::theme::ThemeKind::GrokNight);
            let _ = crate::theme::color_support::set(
                crate::theme::color_support::ColorLevel::TrueColor,
            );

            crate::theme::cache::set_transparent_background(false);
            let solid = Theme::current();
            assert_eq!(solid.canvas, solid.bg_base);
            assert!(!crate::theme::cache::load_transparent_background());

            crate::theme::cache::set_transparent_background(true);
            let see = Theme::current();
            assert!(crate::theme::cache::load_transparent_background());
            // Body is always Reset under transparent mode.
            assert_eq!(see.bg_base, Color::Reset);
            assert_eq!(see.bg_light, Color::Reset);
            // Interaction bands match the opaque Theme::current snapshot.
            assert_eq!(see.bg_dark, solid.bg_dark);
            assert_eq!(see.bg_hover, solid.bg_hover);
            assert_eq!(see.accent_user, solid.accent_user);

            if !matches!(solid.bg_base, Color::Reset) {
                // Colorful terminals: design canvas stays solid for blend math.
                assert_eq!(see.canvas, solid.canvas);
                assert_eq!(see.design_canvas(), solid.bg_base);
                assert_eq!(see.invert_canvas(), Color::Black);
                assert!(see.blend_canvas(see.accent_user, 0.5).is_some());
            }

            crate::theme::cache::set_transparent_background(false);
            let restored = Theme::current();
            assert_eq!(restored.bg_base, solid.bg_base);
            assert_eq!(restored.canvas, solid.canvas);
        });
    }

    /// Discipline guard: theme-relative blends must not pass paint `bg_base`
    /// into raw color helpers. Under transparent mode that paint is
    /// [`Color::Reset`] and the blend silently no-ops.
    ///
    /// Prefer [`Theme::blend_canvas`] / [`Theme::blend`] /
    /// [`Theme::design_canvas`] / [`Theme::solid_paint`].
    ///
    /// Scans pager-render + sibling pager crates. Line comments and the
    /// intentional warning docs in `render/color.rs` are ignored.
    #[test]
    fn no_raw_theme_bg_base_passed_to_color_blend_helpers() {
        use std::path::PathBuf;

        let render_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let crates_root = render_root.parent().expect("pager-render parent");
        let scan_roots = [
            render_root.join("src"),
            crates_root.join("xai-grok-pager/src"),
            crates_root.join("xai-grok-pager-minimal/src"),
            crates_root.join("xai-grok-pager/tests"),
        ];

        // Within ~120 chars of the call open, a paint `.bg_base` is the
        // forbidden blend base. `solid_paint(theme.bg_base)` /
        // `design_canvas()` / `theme.blend(...)` are outside these callees.
        let patterns: &[(&str, regex::Regex)] = &[
            (
                "blend_color",
                regex::Regex::new(r"blend_color\s*\([\s\S]{0,120}?\.bg_base\b").unwrap(),
            ),
            (
                "dim_area",
                regex::Regex::new(r"dim_area\s*\([\s\S]{0,120}?\.bg_base\b").unwrap(),
            ),
            (
                "fade_region",
                regex::Regex::new(r"fade_region\s*\([\s\S]{0,120}?\.bg_base\b").unwrap(),
            ),
            (
                "blend_area",
                regex::Regex::new(r"blend_area\s*\([\s\S]{0,160}?\.bg_base\b").unwrap(),
            ),
        ];

        let mut offenders: Vec<String> = Vec::new();
        for root in &scan_roots {
            if !root.is_dir() {
                continue;
            }
            collect_raw_bg_base_offenders(root, patterns, &mut offenders);
        }

        assert!(
            offenders.is_empty(),
            "raw paint `bg_base` passed into color blend helpers — under \
             transparent mode this silently no-ops. Use Theme::blend_canvas / \
             Theme::design_canvas / Theme::solid_paint instead.\n\n{}",
            offenders.join("\n")
        );
    }

    fn collect_raw_bg_base_offenders(
        root: &std::path::Path,
        patterns: &[(&str, regex::Regex)],
        offenders: &mut Vec<String>,
    ) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_raw_bg_base_offenders(&path, patterns, offenders);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            // Docs in color.rs intentionally show the anti-pattern.
            if path.ends_with("render/color.rs") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            // Drop pure comment lines so doc/example mentions don't fire.
            let code: String = src
                .lines()
                .filter(|l| {
                    let t = l.trim_start();
                    !t.starts_with("//")
                })
                .collect::<Vec<_>>()
                .join("\n");
            for (name, re) in patterns {
                if re.is_match(&code) {
                    offenders.push(format!("  {name}(... .bg_base) in {}", path.display()));
                }
            }
        }
    }
}
