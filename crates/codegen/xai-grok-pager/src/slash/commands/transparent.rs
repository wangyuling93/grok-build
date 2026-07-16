//! `/transparent` (aliases `/transparent-bg`, `/transparency`) — toggle
//! transparent TUI background.
//!
//! When on, every TUI cell uses the host terminal background so translucent
//! terminals (Ghostty, etc.) show through. Persists as
//! `[ui].transparent_background` in config.toml.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

/// Toggle transparent terminal background via `/transparent`.
pub struct TransparentCommand;

impl SlashCommand for TransparentCommand {
    fn name(&self) -> &str {
        "transparent"
    }

    fn aliases(&self) -> &[&str] {
        &["transparent-bg", "transparency"]
    }

    fn description(&self) -> &str {
        "Toggle transparent terminal background"
    }

    /// Minimal already draws on the host canvas; no separate toggle.
    fn available_in_minimal(&self) -> bool {
        false
    }

    fn usage(&self) -> &str {
        "/transparent"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::ToggleTransparentBackground)
    }
}
