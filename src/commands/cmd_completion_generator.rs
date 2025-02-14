use crate::cli::CmdCompletionArgs;
use crate::prelude::*;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io::IsTerminal;
use std::io::Write;

fn gen(shell: Shell, buf: &mut dyn Write) {
    let mut cmd = crate::cli::Cli::command();
    generate(shell, &mut cmd, env!("CARGO_BIN_NAME"), buf);
}

fn detect_shell() -> Result<Shell> {
    use std::env::var;

    if let Ok(shell) = var("SHELL") {
        if shell.ends_with("/fish") {
            return Ok(Shell::Fish);
        } else if shell.ends_with("/bash") {
            return Ok(Shell::Bash);
        } else if shell.ends_with("/zsh") {
            return Ok(Shell::Zsh);
        }
    }

    Err(anyhow!("This shell is unsupported, if this is a mistake set the shell explicitly using the argument"))
}

/// Generates basic completion scripts by `clap_complete`
pub fn shell_completion_generation(cli_args: CmdCompletionArgs) -> Result<()> {
    // prevent terminal text spillage
    if std::io::stdout().is_terminal() {
        println!("This command writes a lot of text, please pipe it into a file");
        std::process::exit(1);
    }

    // use requested shell or detect automatically
    let shell = if let Some(shell) = cli_args.shell {
        shell
    } else {
        detect_shell()?
    };

    gen(shell, &mut std::io::stdout());

    Ok(())
}
