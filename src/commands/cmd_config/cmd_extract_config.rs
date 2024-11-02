use crate::prelude::*;
use crate::cli::cli_config::CmdConfigExtractArgs;
use crate::util::command_extensions::*;

pub fn extract_config(ctx: Context, cli_args: CmdConfigExtractArgs) -> Result<()> {
    // allow dry run to run always
    if !ctx.dry_run {
        let cmd = ctx.engine_command()
            .args(["image", "exists", &cli_args.image])
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        if !cmd.status.success() {
            return Err(anyhow!("Image {} does not exist", cli_args.image));
        }
    }

    let mut cmd = ctx.engine_command();
    cmd.args([
        "run", "--rm", "-it",
        // basically just cat the file, should be pretty portable
        "--entrypoint", "cat",
        &cli_args.image,
        "/config.toml"
    ]);

    if ctx.dry_run {
        cmd.print_escaped_cmd();

        Ok(())
    } else {
        let output = cmd
            .output()
            .expect(crate::ENGINE_ERR_MSG);

        // only print output if command succeds
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));

            Ok(())
        } else {
            return Err(anyhow!("Failed to extract config from image {}", cli_args.image));
        }
    }
}
