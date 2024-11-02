use crate::prelude::*;
use std::process::Command;

#[allow(unused_imports)]
pub mod command_extensions {
    pub use std::process::Command;
    pub use super::{CommandExt, CommandOutputExt};
}

/// Simple extension trait to avoid duplicating code, allow easy conversion to `ExitCode`
pub trait CommandOutputExt {
    /// Convert into `std::process::ExitCode` easily consistantly
    ///
    /// Equal to `ExitCode::from(1)` in case of signal termination (or any exit code larger than 255)
    fn get_code(&self) -> u8;
}

impl CommandOutputExt for std::process::ExitStatus {
    fn get_code(&self) -> u8 {
        // the unwrap_or(1) s are cause even if conversion fails it still failed just termination
        // by signal is larger than 255 that u8 exit code on unix allows
        TryInto::<u8>::try_into(self.code().unwrap_or(1)).unwrap_or(1)
    }
}

impl CommandOutputExt for std::process::Output {
    fn get_code(&self) -> u8 {
        self.status.get_code()
    }
}

pub trait CommandExt {
    /// Prints the command in readable and copy-able format
    fn print_escaped_cmd(&self);

    /// Wrapper around `status` to provide nice anyhow error
    fn run_interactive(&mut self) -> Result<std::process::ExitStatus>;

    /// Wrapper around `output` to provide nice anyhow error
    fn run_get_output(&mut self) -> Result<std::process::Output>;
}

fn format_command(cmd: &Command) -> String {
    let args = cmd.get_args().map(|x| x.to_string_lossy()).collect::<Vec<_>>();
    format!("{} {}", cmd.get_program().to_string_lossy(), args.join(" "))
}

impl CommandExt for Command {
    /// Print the whole command with quotes around each argument
    fn print_escaped_cmd(&self) {
        println!("(CMD) {:?} \\", self.get_program().to_string_lossy());
        let mut iter = self.get_args();
        while let Some(arg) = iter.next() {
            print!("      {:?}", arg.to_string_lossy());

            // do not add backslash on the last argument
            if iter.len() != 0 {
                print!(" \\");
            }

            println!();
        }
    }

    fn run_interactive(&mut self) -> Result<std::process::ExitStatus> {
        let status = self.status()
            .with_context(|| anyhow!("Could not execute {:?}", self.get_program()))?;

        if status.success() {
            Ok(status)
        } else {
            Err(anyhow!("Command {:?} exit with error code {}", format_command(self), status.get_code()))
        }
    }

    fn run_get_output(&mut self) -> Result<std::process::Output> {
        let output = self.output()
            .with_context(|| anyhow!("Could not execute {:?}", self.get_program()))?;

        if output.status.success() {
            Ok(output)
        } else {
            Err(anyhow!("Command {:?} exit with error code {}", format_command(self), output.get_code()))
        }
    }
}
