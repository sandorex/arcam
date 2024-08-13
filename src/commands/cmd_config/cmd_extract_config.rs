use crate::util::{self, Engine};
use crate::cli;
use std::process::{Command, ExitCode};

pub fn extract_config(engine: Engine, dry_run: bool, cli_args: &cli::cli_config::CmdConfigExtractArgs) -> ExitCode {
    let cmd = Command::new(&engine.path)
        .args(["image", "exists", &cli_args.image])
        .output()
        .expect("Could not execute engine");

    if !dry_run && !cmd.status.success() {
        eprintln!("Image {} does not exist", cli_args.image);

        return ExitCode::from(2);
    }

    // basically just cat the file, should be pretty portable
    let args = vec![
        "run".into(), "--rm".into(), "-it".into(),
        "--entrypoint".into(), "cat".into(),
        cli_args.image.clone(),
        "/config.toml".into()
    ];

    if dry_run {
        util::print_cmd_dry_run(&engine, args);

        ExitCode::SUCCESS
    } else {
        let cmd = Command::new(&engine.path)
            .args(args)
            .output()
            .expect("Could not execute engine");

        // only print output if command succeds
        if cmd.status.success() {
            println!("{}", String::from_utf8_lossy(&cmd.stdout));

            ExitCode::SUCCESS
        } else {
            eprintln!("Failed to extract config from image {}", cli_args.image);

            ExitCode::FAILURE
        }
    }
}
