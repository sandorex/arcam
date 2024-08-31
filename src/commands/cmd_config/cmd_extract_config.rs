use crate::ExitResult;
use crate::util::Engine;
use crate::cli::cli_config::CmdConfigExtractArgs;
use crate::util::command_extensions::*;

pub fn extract_config(engine: Engine, dry_run: bool, cli_args: &CmdConfigExtractArgs) -> ExitResult {
    // allow dry run to run always
    if !dry_run {
        let cmd = Command::new(&engine.path)
            .args(["image", "exists", &cli_args.image])
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        if !cmd.status.success() {
            eprintln!("Image {} does not exist", cli_args.image);

            return Err(2);
        }
    }

    let mut cmd = Command::new(&engine.path);
    cmd.args([
        "run", "--rm", "-it",
        // basically just cat the file, should be pretty portable
        "--entrypoint", "cat",
        &cli_args.image,
        "/config.toml"
    ]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        let output = cmd
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        // only print output if command succeds
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));

            Ok(())
        } else {
            eprintln!("Failed to extract config from image {}", cli_args.image);

            Err(1)
        }
    }
}
