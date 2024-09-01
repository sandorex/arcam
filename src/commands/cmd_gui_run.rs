use crate::cli::CmdGuiRunArgs;
use crate::{cli, ExitResult};
use crate::util::command_extensions::*;

pub fn gui_run(dry_run: bool, cli_args: CmdGuiRunArgs) -> ExitResult {
    crate::gui::start_gui(dry_run)?;

    // TODO run commands with env DISPLAY and WAYLAND_DISPLAY
    Ok(())
}
