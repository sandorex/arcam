use crate::{Engine, ExitResult};
use crate::cli::cli_image::CmdImageBuildArgs;
use crate::util::command_extensions::*;
use std::process::Command;

pub fn build_image(engine: &Engine, dry_run: bool, cli_args: CmdImageBuildArgs) -> ExitResult {
    // TODO generate tag timestamp
    let tag = cli_args.tag.unwrap_or_else(|| "test".to_string());

    // TODO check if Containerfile or Dockerfile exist and use that
    let file = cli_args.containerfile.expect("TODO containerfile not set");

    // TODO i do not know if '.' works without shell here??
    let build_context_dir = cli_args.build_dir.unwrap_or_else(|| ".".to_string());

    let mut cmd = Command::new(&engine.path);
    cmd.args([
        "build",
        "--security-opt", "label=disable",
        "-t", &tag,
        "-f", &file,
        &build_context_dir,
    ]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        cmd.status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}
