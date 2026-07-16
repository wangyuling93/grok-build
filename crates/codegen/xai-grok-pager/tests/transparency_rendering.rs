use std::collections::{HashMap, VecDeque};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};
use similar::ChangeTag;
use xai_grok_pager::app::agent::{QueueEntryKind, QueuedPrompt};
use xai_grok_pager::{
    app::bundle::BundleState,
    appearance::LayoutConfig,
    diff::DiffLine,
    scrollback::{EntryId, ScrollbackEntry, block::RenderBlock},
    theme::{Theme, ThemeKind, cache},
    views::{
        block_viewer::BlockViewerPane, queue_pane::QueuePane,
        subagent_catalog_pane::SubagentCatalogPane,
    },
};

fn set_transparency(enabled: bool) {
    cache::set(ThemeKind::GrokNight);
    cache::set_transparent_background(enabled);
}

#[test]
#[serial_test::serial(transparency_rendering)]
fn surviving_edit_viewer_rebuilds_diff_styles_for_transparency() {
    set_transparency(false);
    let block = RenderBlock::edit_with_hunks(
        "src/main.rs",
        vec![vec![
            DiffLine {
                text: "let value = 1;\n".into(),
                lo: 1,
                ln: 1,
                tag: ChangeTag::Delete,
            },
            DiffLine {
                text: "let value = 2;\n".into(),
                lo: 2,
                ln: 1,
                tag: ChangeTag::Insert,
            },
        ]],
    );
    let entry = ScrollbackEntry::with_id(EntryId::new(7), block);
    let mut viewer = BlockViewerPane::for_edit(entry.id, &entry).expect("edit viewer");

    // The viewer and its cached items were created for the opaque paint mode.
    // Switching before the next render must rebuild those items rather than
    // merely letting the final frame scrub erase their old diff bands.
    set_transparency(true);
    let theme = Theme::current();
    let area = Rect::new(0, 0, 100, 4);
    let mut buf = Buffer::empty(area);
    viewer.render_content(area, &mut buf, &entry, false, &[]);

    let mut all_backgrounds_transparent = true;
    let mut delete_foreground_visible = false;
    let mut insert_foreground_visible = false;
    for y in 0..area.height {
        for x in 0..area.width {
            let Some(cell) = buf.cell((x, y)) else {
                continue;
            };
            all_backgrounds_transparent &= cell.bg == Color::Reset;
            delete_foreground_visible |= cell.fg == theme.diff_delete_fg;
            insert_foreground_visible |= cell.fg == theme.diff_insert_fg;
        }
    }
    assert!(all_backgrounds_transparent);
    assert!(
        delete_foreground_visible,
        "deleted lines must gain the bandless red foreground"
    );
    assert!(
        insert_foreground_visible,
        "inserted lines must gain the bandless green foreground"
    );

    set_transparency(false);
}

#[test]
#[serial_test::serial(transparency_rendering)]
fn transparent_queue_hover_keeps_a_row_level_text_cue() {
    set_transparency(true);
    let mut pane = QueuePane::new();
    let local = VecDeque::from([QueuedPrompt::plain(
        1,
        "queued prompt",
        QueueEntryKind::Prompt,
    )]);
    pane.sync_from_merged(&local, &[], None, None, &HashMap::new());

    let area = Rect::new(0, 0, 80, 2);
    let layout = LayoutConfig::default();
    let mut initial = Buffer::empty(area);
    pane.render(area, &mut initial, false, &layout, None, true);

    // Queue content starts at ACCENT + block_pad_left - 1 (2 by default).
    assert!(pane.update_row_hover(2, 0));
    let mut hovered = Buffer::empty(area);
    pane.render(area, &mut hovered, false, &layout, None, true);

    let row_start = hovered.cell((2, 0)).expect("queue row start");
    assert_eq!(row_start.bg, Color::Reset);
    assert!(
        row_start.modifier.contains(Modifier::UNDERLINED),
        "transparent hover must underline the row when no hover band can be painted"
    );

    set_transparency(false);
}

#[test]
#[serial_test::serial(transparency_rendering)]
fn transparent_catalog_selection_refreshes_its_cached_style() {
    set_transparency(false);
    let mut pane = SubagentCatalogPane::new();
    pane.sync_from_bundle(&BundleState {
        has_cache: true,
        personas: vec!["researcher".into()],
        ..BundleState::default()
    });
    let area = Rect::new(0, 0, 80, 4);
    let layout = LayoutConfig::default();
    let mut initial = Buffer::empty(area);
    pane.render(area, &mut initial, true, &layout);
    assert!(pane.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
    assert!(pane.selected_entry().is_some());

    // The pane cached an opaque selection background at construction time.
    // Enabling transparency must refresh that cached style before rendering.
    set_transparency(true);
    let mut buf = Buffer::empty(area);
    pane.render(area, &mut buf, true, &layout);

    let selection_cue = (0..area.width).any(|x| {
        buf.cell((x, 1)).is_some_and(|cell| {
            cell.modifier
                .contains(Modifier::BOLD | Modifier::UNDERLINED)
        })
    });
    assert!(
        selection_cue,
        "transparent catalog selection must retain a visible row cue"
    );

    set_transparency(false);
}
