use crate::cli::CmdCompletionArgs;
use crate::command_extensions::*;
use crate::prelude::*;

/// Used to run autocompletion functions
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ShellCompletionType {
    /// Complete all configs
    Config,

    /// Completes running containers
    Container,
}

/// Prints help that is used for better completion scripts
pub fn shell_completion_helper(ctx: Context, cli_args: CmdCompletionArgs) -> Result<()> {
    match &cli_args.complete.unwrap() {
        // very basic config completer
        ShellCompletionType::Config => {
            for entry in ctx.config_dir().read_dir()?.flatten() {
                let name = entry.file_name();

                // only print .toml files
                if let Some(name) = name.to_string_lossy().strip_suffix(".toml") {
                    println!("@{}", name);
                }
            }
        }

        ShellCompletionType::Container => {
            let output = ctx
                .engine
                .command()
                .args([
                    "container",
                    "ls",
                    "--filter",
                    format!("label={}", crate::APP_NAME).as_str(),
                    "--format",
                    "{{.Names}}",
                ])
                .log_output(log::Level::Debug)?;

            // print the output
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                print!("{}", stdout);
            }
        }
    }

    Ok(())
}
