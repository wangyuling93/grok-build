#!/usr/bin/env python3
"""Regenerate `src/theme/ghostty_catalog.rs` from iTerm2-Color-Schemes ghostty files.

Usage:
  # From a local clone of https://github.com/mbadolato/iTerm2-Color-Schemes
  python3 scripts/generate_ghostty_catalog.py /path/to/iTerm2-Color-Schemes/ghostty

  # Or download the ghostty/ tree into a temp dir and pass that path.

Writes: src/theme/ghostty_catalog.rs (relative to this crate root).
Do not hand-edit the generated file.

First-class hand-mapped themes (Sakura, Aurora) are excluded from the catalog.
Slug collisions with first-class aliases (dark, tokyonight, …) stay in the
catalog; `ThemeKind` in kind.rs maps them to `ghostty-<slug>` config keys
(see COLLIDING_CATALOG / RESERVED_FIRST_CLASS_SLUGS below).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Hand-mapped first-class ThemeKind variants — do not embed catalog duplicates.
FIRST_CLASS_ONLY_SLUGS = frozenset({"sakura", "aurora"})

# Catalog slugs that collide with first-class from_name aliases.
# Must match COLLIDING_CATALOG in src/theme/kind.rs (catalog_slug → config_key).
RESERVED_FIRST_CLASS_SLUGS = frozenset(
    {
        "dark",  # → GrokNight alias; config ghostty-dark
        "rose-pine",  # → RosePineMoon alias
        "rose-pine-moon",
        "tokyonight",  # → TokyoNight
        # Also covered by FIRST_CLASS_ONLY_SLUGS if present in upstream:
        "sakura",
        "aurora",
    }
)


def parse_rgb(s: str) -> tuple[int, int, int] | None:
    s = s.strip()
    m = re.fullmatch(r"#?([0-9a-fA-F]{6})", s)
    if not m:
        return None
    h = m.group(1)
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def slugify(name: str) -> str:
    s = name.strip().lower()
    s = re.sub(r"[^a-z0-9]+", "-", s)
    return s.strip("-")


def parse_ghostty_file(path: Path) -> dict | None:
    text = path.read_text(encoding="utf-8", errors="replace")
    bg = fg = cursor = cursor_text = None
    sel_bg = sel_fg = None
    palette: dict[int, tuple[int, int, int]] = {}

    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            continue
        key, _, val = line.partition("=")
        key, val = key.strip(), val.strip()
        rgb = parse_rgb(val)
        if rgb is None:
            continue
        if key == "background":
            bg = rgb
        elif key == "foreground":
            fg = rgb
        elif key == "cursor-color":
            cursor = rgb
        elif key == "cursor-text":
            cursor_text = rgb
        elif key == "selection-background":
            sel_bg = rgb
        elif key == "selection-foreground":
            sel_fg = rgb
        elif key.startswith("palette"):
            # palette = 0=#rrggbb  or  palette = 0=#rrggbb
            m = re.match(r"palette\s*=\s*(\d+)\s*=\s*(#?[0-9a-fA-F]{6})", line.replace(" ", ""))
            if not m:
                # try "palette = N=#hex"
                m2 = re.match(r"(\d+)=(#?[0-9a-fA-F]{6})", val.replace(" ", ""))
                if m2:
                    idx = int(m2.group(1))
                    pr = parse_rgb(m2.group(2))
                    if pr:
                        palette[idx] = pr
            else:
                idx = int(m.group(1))
                pr = parse_rgb(m.group(2))
                if pr:
                    palette[idx] = pr
        elif re.match(r"palette\s*$", key) or key == "palette":
            pass

    # Alternate palette line form: palette = 0=#aabbcc
    for line in text.splitlines():
        line = line.strip()
        m = re.match(
            r"palette\s*=\s*(\d+)\s*=\s*(#?[0-9a-fA-F]{6})",
            line,
            re.IGNORECASE,
        )
        if m:
            pr = parse_rgb(m.group(2))
            if pr:
                palette[int(m.group(1))] = pr

    if bg is None or fg is None:
        return None
    if cursor is None:
        cursor = fg
    if cursor_text is None:
        cursor_text = bg
    if sel_bg is None:
        sel_bg = fg
    if sel_fg is None:
        sel_fg = bg
    colors = []
    for i in range(16):
        colors.append(palette.get(i, bg if i == 0 else fg))

    display = path.stem
    return {
        "display": display,
        "slug": slugify(display),
        "background": bg,
        "foreground": fg,
        "cursor": cursor,
        "cursor_text": cursor_text,
        "selection_background": sel_bg,
        "selection_foreground": sel_fg,
        "palette": colors,
    }


def rust_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def fmt_rgb(t: tuple[int, int, int]) -> str:
    return f"({t[0]}, {t[1]}, {t[2]})"


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    src = Path(sys.argv[1])
    if not src.is_dir():
        print(f"not a directory: {src}", file=sys.stderr)
        return 1

    schemes = []
    skipped_first_class = []
    for path in sorted(src.iterdir()):
        if not path.is_file():
            continue
        parsed = parse_ghostty_file(path)
        if not parsed:
            continue
        if parsed["slug"] in FIRST_CLASS_ONLY_SLUGS:
            skipped_first_class.append(parsed["slug"])
            continue
        schemes.append(parsed)

    if len(schemes) < 100:
        print(f"only parsed {len(schemes)} schemes; aborting", file=sys.stderr)
        return 1

    colliding = sorted(s["slug"] for s in schemes if s["slug"] in RESERVED_FIRST_CLASS_SLUGS)
    if colliding:
        print(
            f"note: {len(colliding)} catalog slug(s) collide with first-class aliases "
            f"(config keys ghostty-… in kind.rs): {', '.join(colliding)}",
            file=sys.stderr,
        )

    crate_root = Path(__file__).resolve().parents[1]
    out_path = crate_root / "src" / "theme" / "ghostty_catalog.rs"

    lines = [
        "//! Auto-generated Ghostty terminal color schemes.",
        "//!",
        "//! Source: mbadolato/iTerm2-Color-Schemes `ghostty/` (Apache-2.0).",
        "//! Do not hand-edit. Regenerate with:",
        "//!   python3 scripts/generate_ghostty_catalog.py /path/to/iTerm2-Color-Schemes/ghostty",
        "//!",
        "//! Excludes first-class hand-mapped slugs (sakura, aurora).",
        f"//! Scheme count: {len(schemes)}",
        "",
        "/// One Ghostty/iTerm2 terminal color scheme.",
        "#[derive(Debug, Clone, Copy)]",
        "pub struct GhosttyScheme {",
        "    pub display: &'static str,",
        "    pub slug: &'static str,",
        "    pub background: (u8, u8, u8),",
        "    pub foreground: (u8, u8, u8),",
        "    pub cursor: (u8, u8, u8),",
        "    pub cursor_text: (u8, u8, u8),",
        "    pub selection_background: (u8, u8, u8),",
        "    pub selection_foreground: (u8, u8, u8),",
        "    pub palette: [(u8, u8, u8); 16],",
        "}",
        "",
        "pub static GHOSTTY_SCHEMES: &[GhosttyScheme] = &[",
    ]

    for s in schemes:
        pal = ", ".join(fmt_rgb(c) for c in s["palette"])
        lines.append("    GhosttyScheme {")
        lines.append(f'        display: "{rust_escape(s["display"])}",')
        lines.append(f'        slug: "{rust_escape(s["slug"])}",')
        lines.append(f"        background: {fmt_rgb(s['background'])},")
        lines.append(f"        foreground: {fmt_rgb(s['foreground'])},")
        lines.append(f"        cursor: {fmt_rgb(s['cursor'])},")
        lines.append(f"        cursor_text: {fmt_rgb(s['cursor_text'])},")
        lines.append(f"        selection_background: {fmt_rgb(s['selection_background'])},")
        lines.append(f"        selection_foreground: {fmt_rgb(s['selection_foreground'])},")
        lines.append(f"        palette: [{pal}],")
        lines.append("    },")

    lines.extend(
        [
            "];",
            "",
            "use std::collections::HashMap;",
            "use std::sync::OnceLock;",
            "",
            "fn slug_index() -> &'static HashMap<&'static str, u16> {",
            "    static INDEX: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();",
            "    INDEX.get_or_init(|| {",
            "        GHOSTTY_SCHEMES",
            "            .iter()",
            "            .enumerate()",
            "            .map(|(i, s)| (s.slug, i as u16))",
            "            .collect()",
            "    })",
            "}",
            "",
            "fn display_index() -> &'static HashMap<String, u16> {",
            "    static INDEX: OnceLock<HashMap<String, u16>> = OnceLock::new();",
            "    INDEX.get_or_init(|| {",
            "        GHOSTTY_SCHEMES",
            "            .iter()",
            "            .enumerate()",
            "            .map(|(i, s)| (s.display.to_ascii_lowercase(), i as u16))",
            "            .collect()",
            "    })",
            "}",
            "",
            "#[must_use]",
            "pub fn scheme_by_slug(slug: &str) -> Option<(u16, &'static GhosttyScheme)> {",
            "    let lower = slug.to_ascii_lowercase();",
            "    let i = *slug_index().get(lower.as_str())?;",
            "    GHOSTTY_SCHEMES.get(i as usize).map(|s| (i, s))",
            "}",
            "",
            "#[must_use]",
            "pub fn scheme_by_display(name: &str) -> Option<(u16, &'static GhosttyScheme)> {",
            "    let key = name.to_ascii_lowercase();",
            "    let i = *display_index().get(&key)?;",
            "    GHOSTTY_SCHEMES.get(i as usize).map(|s| (i, s))",
            "}",
            "",
        ]
    )

    out_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {len(schemes)} schemes to {out_path}")
    if skipped_first_class:
        print(f"skipped first-class-only: {', '.join(sorted(set(skipped_first_class)))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
