//! [`ThemeKind`] — identity, parsing, encode/decode, and picker catalogs.
//!
//! Built-in themes (including hand-mapped Sakura / Aurora) are first-class
//! enum variants. All other Ghostty / iTerm2 schemes are [`ThemeKind::Ghostty`]
//! indices into [`GHOSTTY_SCHEMES`]. First-class-only slugs are excluded from
//! the catalog at generation time (`scripts/generate_ghostty_catalog.py`).

use std::sync::OnceLock;

use super::color_support;
use super::ghostty;
use super::ghostty_catalog::{self, GHOSTTY_SCHEMES};
use super::tokyonight::Theme;

/// Catalog schemes whose bare slug collides with a first-class name/alias.
///
/// `(catalog_slug, config_key)` — config persists as `config_key` so first-class
/// arms keep `"dark"` / `"tokyonight"` / etc. Single table for lookup + naming.
/// Keep in sync with `RESERVED_FIRST_CLASS_SLUGS` in
/// `scripts/generate_ghostty_catalog.py`.
const COLLIDING_CATALOG: &[(&str, &str)] = &[
    ("dark", "ghostty-dark"),
    ("rose-pine", "ghostty-rose-pine"),
    ("rose-pine-moon", "ghostty-rose-pine-moon"),
    ("tokyonight", "ghostty-tokyonight"),
];

/// One row in Settings / slash theme pickers (canonical config name + UI copy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePickerRow {
    pub canonical: &'static str,
    pub display: &'static str,
    pub description: &'static str,
}

/// Available theme variants.
///
/// Built-in themes are first-class. [`ThemeKind::Ghostty`] indexes the
/// embedded Ghostty / iTerm2 catalog ([`GHOSTTY_SCHEMES`], ~590 schemes).
///
/// Encoding for the theme cache uses [`ThemeKind::encode`] / [`decode`]
/// (not C-like discriminants) so Ghostty indices fit in a `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeKind {
    GrokNight,
    GrokDay,
    TokyoNight,
    RosePineMoon,
    OscuraMidnight,
    /// Ghostty Sakura — first-class (hand-mapped TUI chrome).
    Sakura,
    /// Ghostty Aurora — first-class (hand-mapped TUI chrome).
    Aurora,
    /// Meta-variant: follow system dark/light appearance.
    ///
    /// Never stored as the live palette — resolved to a concrete theme
    /// at startup and on appearance changes. Excluded from [`ALL`].
    Auto,
    /// Index into [`GHOSTTY_SCHEMES`].
    Ghostty(u16),
}

impl ThemeKind {
    /// First-class built-in themes (excluding `Auto` and the Ghostty catalog).
    pub const ALL: &[ThemeKind] = &[
        ThemeKind::GrokNight,
        ThemeKind::GrokDay,
        ThemeKind::TokyoNight,
        ThemeKind::RosePineMoon,
        ThemeKind::OscuraMidnight,
        ThemeKind::Sakura,
        ThemeKind::Aurora,
    ];

    /// Number of Ghostty catalog schemes shown in pickers.
    #[must_use]
    pub fn catalog_picker_count() -> usize {
        GHOSTTY_SCHEMES.len()
    }

    /// Map a catalog index to a kind (or [`ThemeKind::GrokNight`] if out of range).
    #[must_use]
    pub fn from_catalog_index(index: u16) -> Self {
        if (index as usize) < GHOSTTY_SCHEMES.len() {
            Self::Ghostty(index)
        } else {
            Self::GrokNight
        }
    }

    /// Built-in + Ghostty catalog kinds selectable on this terminal.
    ///
    /// Without truecolor, only GrokNight / GrokDay. With truecolor, built-ins
    /// first, then the full catalog. Cached for the process.
    #[must_use]
    pub fn available() -> &'static [ThemeKind] {
        if !color_support::detect().has_truecolor() {
            const NO_TRUECOLOR: &[ThemeKind] = &[ThemeKind::GrokNight, ThemeKind::GrokDay];
            return NO_TRUECOLOR;
        }
        static FULL: OnceLock<Vec<ThemeKind>> = OnceLock::new();
        FULL.get_or_init(build_full_available).as_slice()
    }

    /// Settings / config picker: `auto` then every concrete theme (no truecolor filter).
    ///
    /// Single source of truth for theme enum rows — Settings maps these to
    /// `EnumChoice` without re-listing built-ins or re-filtering the catalog.
    #[must_use]
    pub fn settings_theme_rows() -> &'static [ThemePickerRow] {
        static ROWS: OnceLock<Vec<ThemePickerRow>> = OnceLock::new();
        ROWS.get_or_init(|| {
            let mut out = Vec::with_capacity(1 + ThemeKind::ALL.len() + GHOSTTY_SCHEMES.len());
            out.push(ThemePickerRow {
                canonical: "auto",
                display: "Auto",
                description: "Follow system dark/light appearance.",
            });
            out.extend(concrete_picker_rows());
            out
        })
        .as_slice()
    }

    /// Concrete themes only (settings `auto_dark` / `auto_light` — no `auto` row).
    #[must_use]
    pub fn settings_concrete_theme_rows() -> &'static [ThemePickerRow] {
        let all = Self::settings_theme_rows();
        debug_assert_eq!(all.first().map(|r| r.canonical), Some("auto"));
        &all[1..]
    }

    /// Config / slash canonical name (slug).
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::GrokNight => "groknight",
            Self::TokyoNight => "tokyonight",
            Self::GrokDay => "grokday",
            Self::RosePineMoon => "rosepine-moon",
            Self::OscuraMidnight => "oscura-midnight",
            Self::Sakura => "sakura",
            Self::Aurora => "aurora",
            Self::Auto => "auto",
            Self::Ghostty(i) => GHOSTTY_SCHEMES
                .get(i as usize)
                .map(catalog_config_slug)
                .unwrap_or("groknight"),
        }
    }

    /// Human label for pickers (may contain spaces).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::GrokNight => "Grok Night",
            Self::GrokDay => "Grok Day",
            Self::TokyoNight => "Tokyo Night",
            Self::RosePineMoon => "Rose Pine Moon",
            Self::OscuraMidnight => "Oscura Midnight",
            Self::Sakura => "Sakura",
            Self::Aurora => "Aurora",
            Self::Auto => "Auto",
            Self::Ghostty(i) => GHOSTTY_SCHEMES
                .get(i as usize)
                .map(|s| s.display)
                .unwrap_or("Grok Night"),
        }
    }

    /// Short description for settings chooser sub-text.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::Auto => "Follow system dark/light appearance.",
            Self::GrokNight => "Neutral dark with magenta accent.",
            Self::GrokDay => "Light theme for bright environments.",
            Self::TokyoNight => "Dark + blue-tinted; needs truecolor.",
            Self::RosePineMoon => "Muted dark with mauve accents; needs truecolor.",
            Self::OscuraMidnight => "Deep dark with warm accents; needs truecolor.",
            Self::Sakura => {
                "Ghostty Sakura — dark plum with magenta blossom; needs truecolor."
            }
            Self::Aurora => {
                "Ghostty Aurora — dark slate with amber/cyan accents; needs truecolor."
            }
            Self::Ghostty(_) => "Ghostty terminal color scheme; needs truecolor.",
        }
    }

    /// Whether this theme requires truecolor (24-bit RGB) to look correct.
    #[must_use]
    pub fn requires_truecolor(self) -> bool {
        match self {
            Self::GrokNight | Self::GrokDay | Self::Auto => false,
            Self::TokyoNight
            | Self::RosePineMoon
            | Self::OscuraMidnight
            | Self::Sakura
            | Self::Aurora
            | Self::Ghostty(_) => true,
        }
    }

    /// Parse a theme name (case-insensitive). All string→ThemeKind
    /// conversions must go through this function.
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        match lower.as_str() {
            "auto" | "system" => Some(Self::Auto),
            "groknight" | "grok-night" | "dark" => Some(Self::GrokNight),
            "tokyonight" | "tokyo-night" | "tokyo" => Some(Self::TokyoNight),
            "grokday" | "grok-day" | "light" | "day" => Some(Self::GrokDay),
            "rosepine" | "rose-pine" | "rosepine-moon" | "rose-pine-moon" => {
                Some(Self::RosePineMoon)
            }
            "oscura" | "oscura-midnight" => Some(Self::OscuraMidnight),
            "sakura" | "cherry" | "cherry-blossom" => Some(Self::Sakura),
            "aurora" | "northern-lights" => Some(Self::Aurora),
            other => Self::from_catalog_name(other, name),
        }
    }

    /// Resolve a Ghostty catalog name (slug, `ghostty-*` config key, or display).
    fn from_catalog_name(lower: &str, original: &str) -> Option<Self> {
        // Disambiguated config keys from [`COLLIDING_CATALOG`].
        if let Some(&(slug, _)) = COLLIDING_CATALOG.iter().find(|&&(_, config)| config == lower)
        {
            return ghostty_catalog::scheme_by_slug(slug).map(|(i, _)| Self::Ghostty(i));
        }
        if let Some((i, scheme)) = ghostty_catalog::scheme_by_slug(lower) {
            // Bare colliding slugs are first-class (handled in `from_name`);
            // catalog copies use the table's config_key only.
            if is_colliding_catalog_slug(scheme.slug) {
                return None;
            }
            return Some(Self::Ghostty(i));
        }
        ghostty_catalog::scheme_by_display(original).map(|(i, _)| Self::Ghostty(i))
    }

    /// Whether this is the meta "auto" variant (resolved at runtime).
    #[must_use]
    pub fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Unquantized palette for this kind (design RGB, no paint-mode flags).
    ///
    /// `Auto` maps to [`Theme::groknight`] — the same nominal default the
    /// cache uses before system appearance resolves a concrete theme.
    #[must_use]
    pub fn to_theme(self) -> Theme {
        match self {
            Self::GrokNight | Self::Auto => Theme::groknight(),
            Self::GrokDay => Theme::grokday(),
            Self::TokyoNight => Theme::tokyonight(),
            Self::RosePineMoon => Theme::rosepine_moon(),
            Self::OscuraMidnight => Theme::oscura_midnight(),
            Self::Sakura => Theme::sakura(),
            Self::Aurora => Theme::aurora(),
            Self::Ghostty(i) => ghostty::theme_from_ghostty_index(i),
        }
    }

    /// Designed dark/light polarity (cheap for catalog indices).
    #[must_use]
    pub fn is_dark(self) -> bool {
        match self {
            Self::Ghostty(i) => ghostty::catalog_index_is_dark(i),
            other => other.to_theme().is_dark(),
        }
    }

    /// Pack for the atomic theme cache (`u32`).
    #[must_use]
    pub fn encode(self) -> u32 {
        match self {
            Self::GrokNight => 0,
            Self::GrokDay => 1,
            Self::TokyoNight => 2,
            Self::RosePineMoon => 3,
            Self::Auto => 4,
            Self::OscuraMidnight => 5,
            Self::Sakura => 6,
            Self::Aurora => 7,
            // High bit marks Ghostty catalog index in the low 16 bits.
            Self::Ghostty(i) => 0x8000_0000 | u32::from(i),
        }
    }

    /// Unpack from the atomic theme cache.
    #[must_use]
    pub fn decode(raw: u32) -> Self {
        if raw & 0x8000_0000 != 0 {
            let i = (raw & 0xFFFF) as u16;
            return Self::from_catalog_index(i);
        }
        match raw {
            0 => Self::GrokNight,
            1 => Self::GrokDay,
            2 => Self::TokyoNight,
            3 => Self::RosePineMoon,
            4 => Self::Auto,
            5 => Self::OscuraMidnight,
            6 => Self::Sakura,
            7 => Self::Aurora,
            _ => Self::GrokNight,
        }
    }
}

/// `FromStr` wrapper around [`ThemeKind::from_name`].
impl std::str::FromStr for ThemeKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s).ok_or(())
    }
}

#[must_use]
fn is_colliding_catalog_slug(slug: &str) -> bool {
    COLLIDING_CATALOG.iter().any(|&(s, _)| s == slug)
}

/// Config slug for a catalog scheme (`ghostty-<slug>` when reserved).
#[must_use]
fn catalog_config_slug(scheme: &ghostty_catalog::GhosttyScheme) -> &'static str {
    COLLIDING_CATALOG
        .iter()
        .find(|&&(s, _)| s == scheme.slug)
        .map(|&(_, config)| config)
        .unwrap_or(scheme.slug)
}

/// First-class built-ins, then every catalog index as [`ThemeKind::Ghostty`].
fn concrete_theme_kinds() -> impl Iterator<Item = ThemeKind> {
    ThemeKind::ALL
        .iter()
        .copied()
        .chain((0..GHOSTTY_SCHEMES.len() as u16).map(ThemeKind::Ghostty))
}

fn build_full_available() -> Vec<ThemeKind> {
    concrete_theme_kinds().collect()
}

fn concrete_picker_rows() -> Vec<ThemePickerRow> {
    concrete_theme_kinds()
        .map(|kind| ThemePickerRow {
            canonical: kind.display_name(),
            display: kind.label(),
            description: kind.description(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color_support;

    #[test]
    fn from_name_auto() {
        assert_eq!(ThemeKind::from_name("auto"), Some(ThemeKind::Auto));
    }

    #[test]
    fn from_name_system() {
        assert_eq!(ThemeKind::from_name("system"), Some(ThemeKind::Auto));
    }

    #[test]
    fn from_name_auto_case_insensitive() {
        assert_eq!(ThemeKind::from_name("AUTO"), Some(ThemeKind::Auto));
        assert_eq!(ThemeKind::from_name("Auto"), Some(ThemeKind::Auto));
        assert_eq!(ThemeKind::from_name("SYSTEM"), Some(ThemeKind::Auto));
    }

    #[test]
    fn display_name_auto() {
        assert_eq!(ThemeKind::Auto.display_name(), "auto");
    }

    #[test]
    fn is_auto_returns_true_for_auto() {
        assert!(ThemeKind::Auto.is_auto());
    }

    #[test]
    fn is_auto_returns_false_for_concrete_variants() {
        assert!(!ThemeKind::GrokNight.is_auto());
        assert!(!ThemeKind::GrokDay.is_auto());
        assert!(!ThemeKind::TokyoNight.is_auto());
        assert!(!ThemeKind::RosePineMoon.is_auto());
        assert!(!ThemeKind::OscuraMidnight.is_auto());
        assert!(!ThemeKind::Sakura.is_auto());
        assert!(!ThemeKind::Aurora.is_auto());
    }

    #[test]
    fn all_excludes_auto() {
        assert!(!ThemeKind::ALL.contains(&ThemeKind::Auto));
    }

    #[test]
    fn available_excludes_auto() {
        assert!(!ThemeKind::available().contains(&ThemeKind::Auto));
    }

    #[test]
    fn auto_does_not_require_truecolor() {
        assert!(!ThemeKind::Auto.requires_truecolor());
    }

    #[test]
    fn from_name_concrete_variants_still_work() {
        assert_eq!(
            ThemeKind::from_name("groknight"),
            Some(ThemeKind::GrokNight)
        );
        assert_eq!(ThemeKind::from_name("dark"), Some(ThemeKind::GrokNight));
        assert_eq!(ThemeKind::from_name("grokday"), Some(ThemeKind::GrokDay));
        assert_eq!(ThemeKind::from_name("light"), Some(ThemeKind::GrokDay));
        assert_eq!(
            ThemeKind::from_name("tokyonight"),
            Some(ThemeKind::TokyoNight)
        );
        assert_eq!(
            ThemeKind::from_name("rosepine"),
            Some(ThemeKind::RosePineMoon)
        );
        assert_eq!(
            ThemeKind::from_name("oscura"),
            Some(ThemeKind::OscuraMidnight)
        );
        assert_eq!(
            ThemeKind::from_name("oscura-midnight"),
            Some(ThemeKind::OscuraMidnight)
        );
        assert_eq!(ThemeKind::from_name("sakura"), Some(ThemeKind::Sakura));
        assert_eq!(ThemeKind::from_name("cherry"), Some(ThemeKind::Sakura));
        assert_eq!(ThemeKind::from_name("aurora"), Some(ThemeKind::Aurora));
    }

    #[test]
    fn from_str_matches_from_name_for_all_canonicals() {
        let cases = [
            ("auto", ThemeKind::Auto),
            ("system", ThemeKind::Auto),
            ("groknight", ThemeKind::GrokNight),
            ("grok-night", ThemeKind::GrokNight),
            ("dark", ThemeKind::GrokNight),
            ("tokyonight", ThemeKind::TokyoNight),
            ("tokyo-night", ThemeKind::TokyoNight),
            ("tokyo", ThemeKind::TokyoNight),
            ("grokday", ThemeKind::GrokDay),
            ("grok-day", ThemeKind::GrokDay),
            ("light", ThemeKind::GrokDay),
            ("day", ThemeKind::GrokDay),
            ("rosepine", ThemeKind::RosePineMoon),
            ("rose-pine", ThemeKind::RosePineMoon),
            ("rosepine-moon", ThemeKind::RosePineMoon),
            ("rose-pine-moon", ThemeKind::RosePineMoon),
            ("oscura", ThemeKind::OscuraMidnight),
            ("oscura-midnight", ThemeKind::OscuraMidnight),
            ("sakura", ThemeKind::Sakura),
            ("cherry", ThemeKind::Sakura),
            ("cherry-blossom", ThemeKind::Sakura),
            ("aurora", ThemeKind::Aurora),
            ("northern-lights", ThemeKind::Aurora),
        ];
        for (name, expected) in cases {
            assert_eq!(
                name.parse::<ThemeKind>(),
                Ok(expected),
                "name `{name}` must parse to {expected:?}",
            );
            assert_eq!(
                name.to_uppercase().parse::<ThemeKind>(),
                Ok(expected),
                "name `{name}` (upper) must parse to {expected:?}",
            );
        }
        assert_eq!("nonexistent".parse::<ThemeKind>(), Err(()));
        assert_eq!("".parse::<ThemeKind>(), Err(()));
    }

    #[test]
    fn ghostty_catalog_from_name_and_encode_roundtrip() {
        let dracula = ThemeKind::from_name("dracula").expect("dracula in catalog");
        assert!(matches!(dracula, ThemeKind::Ghostty(_)));
        assert_eq!(dracula.display_name(), "dracula");
        assert_eq!(ThemeKind::decode(dracula.encode()), dracula);

        // Sakura / Aurora are first-class only (not duplicated in the catalog).
        assert_eq!(ThemeKind::from_name("sakura"), Some(ThemeKind::Sakura));
        assert_eq!(ThemeKind::from_name("aurora"), Some(ThemeKind::Aurora));
        assert!(ghostty_catalog::scheme_by_slug("sakura").is_none());
        assert!(ghostty_catalog::scheme_by_slug("aurora").is_none());

        let avail = ThemeKind::available();
        if color_support::detect().has_truecolor() {
            assert!(avail.len() > 500);
            assert_eq!(&avail[..ThemeKind::ALL.len()], ThemeKind::ALL);
            assert_eq!(
                avail.len(),
                ThemeKind::ALL.len() + ThemeKind::catalog_picker_count()
            );
            assert_eq!(ThemeKind::catalog_picker_count(), GHOSTTY_SCHEMES.len());
        }

        assert_eq!(ThemeKind::GrokNight.encode(), 0);
        assert_eq!(ThemeKind::Aurora.encode(), 7);
        if let ThemeKind::Ghostty(i) = dracula {
            assert_eq!(dracula.encode(), 0x8000_0000 | u32::from(i));
        }

        // Settings rows: auto + same concrete set as available (on truecolor).
        let rows = ThemeKind::settings_theme_rows();
        assert_eq!(rows[0].canonical, "auto");
        assert_eq!(
            ThemeKind::settings_concrete_theme_rows().len(),
            rows.len() - 1
        );
        assert_eq!(
            ThemeKind::settings_concrete_theme_rows()[0].canonical,
            "groknight"
        );
    }

    #[test]
    fn is_dark_classifies_built_in_themes() {
        assert!(Theme::groknight().is_dark());
        assert!(Theme::tokyonight().is_dark());
        assert!(Theme::rosepine_moon().is_dark());
        assert!(Theme::oscura_midnight().is_dark());
        assert!(Theme::sakura().is_dark());
        assert!(Theme::aurora().is_dark());
        assert!(!Theme::grokday().is_dark());
        for &kind in ThemeKind::ALL {
            assert_eq!(
                kind.is_dark(),
                kind.to_theme().is_dark(),
                "{kind:?} ThemeKind::is_dark must match palette"
            );
        }
        assert!(ThemeKind::Auto.is_dark());
        if let Some(kind) = ThemeKind::from_name("dracula") {
            assert!(kind.is_dark());
            assert_eq!(kind.is_dark(), kind.to_theme().is_dark());
        }
        if let Some(kind) = ThemeKind::from_name("github-light-default") {
            assert!(!kind.is_dark());
            assert_eq!(kind.is_dark(), kind.to_theme().is_dark());
        }
    }

    #[test]
    fn settings_rows_match_kind_labels() {
        for row in ThemeKind::settings_concrete_theme_rows() {
            let kind = ThemeKind::from_name(row.canonical).expect(row.canonical);
            assert_eq!(kind.display_name(), row.canonical);
            assert_eq!(kind.label(), row.display);
        }
    }

    #[test]
    fn colliding_catalog_slugs_use_ghostty_prefix() {
        // First-class aliases keep priority.
        assert_eq!(ThemeKind::from_name("dark"), Some(ThemeKind::GrokNight));
        assert_eq!(
            ThemeKind::from_name("tokyonight"),
            Some(ThemeKind::TokyoNight)
        );
        // Every collision table entry round-trips through from_name / display_name.
        for &(slug, config) in COLLIDING_CATALOG {
            let kind = ThemeKind::from_name(config).unwrap_or_else(|| panic!("{config}"));
            assert!(matches!(kind, ThemeKind::Ghostty(_)), "{config}");
            assert_eq!(kind.display_name(), config);
            let (i, scheme) = ghostty_catalog::scheme_by_slug(slug).expect(slug);
            assert_eq!(kind, ThemeKind::Ghostty(i));
            assert_eq!(catalog_config_slug(scheme), config);
        }
        assert_eq!(
            ThemeKind::from_name("ghostty-dark").map(|k| k.label()),
            Some("Dark+")
        );
    }

    /// After a catalog regen, any slug that `from_name` steals for a first-class
    /// theme must be declared in [`COLLIDING_CATALOG`] (or excluded at gen time
    /// via `FIRST_CLASS_ONLY_SLUGS`). Prevents silent unreachable schemes.
    #[test]
    fn catalog_slugs_colliding_with_first_class_are_declared() {
        for scheme in GHOSTTY_SCHEMES {
            let Some(kind) = ThemeKind::from_name(scheme.slug) else {
                panic!("catalog slug `{}` is unreachable via from_name", scheme.slug);
            };
            match kind {
                ThemeKind::Ghostty(i) => {
                    assert_eq!(
                        GHOSTTY_SCHEMES[i as usize].slug,
                        scheme.slug,
                        "slug `{}` must map to its own index",
                        scheme.slug
                    );
                }
                other => {
                    assert!(
                        is_colliding_catalog_slug(scheme.slug),
                        "catalog slug `{}` resolves to first-class {other:?}; \
                         add it to COLLIDING_CATALOG or FIRST_CLASS_ONLY_SLUGS",
                        scheme.slug
                    );
                    let config = catalog_config_slug(scheme);
                    let via_config = ThemeKind::from_name(config)
                        .unwrap_or_else(|| panic!("config key `{config}` must resolve"));
                    assert!(
                        matches!(via_config, ThemeKind::Ghostty(_)),
                        "{config} should select the catalog scheme"
                    );
                }
            }
        }
    }

    #[test]
    fn concrete_kinds_match_available_on_truecolor() {
        if !color_support::detect().has_truecolor() {
            return;
        }
        let kinds: Vec<_> = concrete_theme_kinds().collect();
        assert_eq!(kinds.as_slice(), ThemeKind::available());
        assert_eq!(
            ThemeKind::settings_concrete_theme_rows().len(),
            kinds.len()
        );
    }
}
