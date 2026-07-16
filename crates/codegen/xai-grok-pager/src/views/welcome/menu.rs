//! Menu component — renders shortcut key menus.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::theme::Theme;

use super::logo::logo_visual_width;

/// Render the welcome menu rows as `label … shortcut`, padded within each row.
/// Returns the Rect for each item row (for hit-testing clicks and hover).
pub fn render_menu(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    items: &[(&str, &str)],
    selected: Option<usize>,
    mouse_pos: Option<(u16, u16)>,
    min_width_hint: u16,
) -> Vec<Rect> {
    let label_style = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(theme.gray_bright);

    // Width: label + gap + key. Keep a 4-col gap between label and key for
    // readability.
    let content_min: u16 = items
        .iter()
        .map(|(key, label)| (key.len() + label.len() + 4) as u16)
        .max()
        .unwrap_or(0);
    let menu_width = logo_visual_width(area.height)
        .max(30)
        .max(content_min)
        .max(min_width_hint);

    let [_, menu_centered, _] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(menu_width),
        Constraint::Min(0),
    ])
    .flex(Flex::Center)
    .areas(area);

    let mut rects = Vec::with_capacity(items.len());
    let mut y = menu_centered.y;
    for (i, (key, label)) in items.iter().enumerate() {
        if y >= menu_centered.y + menu_centered.height {
            break;
        }

        let is_selected = selected == Some(i);
        let key_width = key.len() as u16;
        let label_len = label.len() as u16;

        let row_rect = Rect {
            x: menu_centered.x,
            y,
            width: menu_centered.width,
            height: 1,
        };
        rects.push(row_rect);

        // Label, flush with the left edge of the menu column.
        buf.set_span(
            menu_centered.x,
            y,
            &Span::styled(*label, label_style),
            label_len,
        );

        // Key shortcut flush with the right edge of the menu column.
        buf.set_span(
            menu_centered.x + menu_centered.width - key_width,
            y,
            &Span::styled(*key, key_style),
            key_width,
        );

        // [x] dismiss affordance restyling (for the import row)
        if let Some(x_offset) = key.rfind("[x]") {
            let key_x_start = menu_centered.x + menu_centered.width - key_width;
            let dismiss_start = key_x_start + x_offset as u16;
            let dismiss_end = dismiss_start + 3;
            let mouse_on_dismiss = mouse_pos
                .is_some_and(|(mx, my)| my == y && mx >= dismiss_start && mx < dismiss_end);
            let dismiss_color = if mouse_on_dismiss {
                theme.text_primary
            } else {
                theme.gray_bright
            };
            let dismiss_style = Style::default()
                .fg(dismiss_color)
                .add_modifier(Modifier::BOLD);
            for (offset, ch) in "[x]".chars().enumerate() {
                let col = dismiss_start + offset as u16;
                if let Some(cell) = buf.cell_mut((col, y)) {
                    cell.set_char(ch);
                    cell.set_style(dismiss_style);
                }
            }
        }

        // Single selection paint path — Theme owns transparent-safe cues.
        if is_selected {
            buf.set_style(
                row_rect,
                theme.selection_overlay_style(theme.bg_highlight, true),
            );
        }

        y += 1;
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn transparent_selected_row_uses_non_background_cue() {
        let area = Rect::new(0, 0, 80, 3);
        let mut buf = Buffer::empty(area);
        let theme = Theme::groknight().transparent_elevated();
        let rects = render_menu(
            area,
            &mut buf,
            &theme,
            &[("Enter", "Start"), ("Esc", "Quit")],
            Some(0),
            None,
            0,
        );

        let selected = &buf[(rects[0].x, rects[0].y)];
        assert_eq!(selected.bg, Color::Reset);
        assert!(
            selected.modifier.contains(Modifier::UNDERLINED),
            "selected transparent menu row needs a non-background cue: {selected:?}"
        );

        let unselected = &buf[(rects[1].x, rects[1].y)];
        assert!(
            !unselected.modifier.contains(Modifier::UNDERLINED),
            "unselected menu row must not inherit the selection cue: {unselected:?}"
        );
    }
}
