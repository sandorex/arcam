use crate::prelude::*;
use std::process::{Child, Command, ExitStatus, Output};

#[allow(unused_imports)]
pub mod command_extensions {
    pub use super::{CommandExt, CommandOutputExt};
    pub use std::process::Command;
}

/// Simple extension trait to avoid duplicating code, allow easy conversion to `ExitCode`
pub trait CommandOutputExt {
    /// Convert into `std::process::ExitCode` easily consistantly
    ///
    /// Equal to `ExitCode::from(1)` in case of signal termination (or any exit code larger than 255)
    fn get_code(&self) -> u8;
}

impl CommandOutputExt for ExitStatus {
    fn get_code(&self) -> u8 {
        // the unwrap_or(1) s are cause even if conversion fails it still failed just termination
        // by signal is larger than 255 that u8 exit code on unix allows
        TryInto::<u8>::try_into(self.code().unwrap_or(1)).unwrap_or(1)
    }
}

impl CommandOutputExt for Output {
    fn get_code(&self) -> u8 {
        self.status.get_code()
    }
}

pub trait CommandExt {
    /// Prints the command in readable and copy-able format
    #[deprecated]
    fn print_escaped_cmd(&self);

    fn get_full_command(&self) -> String;

    /// Logs command and output after running `output`
    fn log_output(&mut self, level: log::Level) -> std::io::Result<Output>;

    /// Same as `log_output` but anyhow error
    fn log_output_anyhow(&mut self, level: log::Level) -> Result<Output>;

    /// Logs command and output after running `status`
    fn log_status(&mut self, level: log::Level) -> std::io::Result<ExitStatus>;

    /// Same as `log_status` but anyhow error
    fn log_status_anyhow(&mut self, level: log::Level) -> Result<ExitStatus>;

    /// Logs command and output after running `spawn`
    fn log_spawn(&mut self, level: log::Level) -> std::io::Result<Child>;

    /// Same as `log_spawn` but anyhow error
    fn log_spawn_anyhow(&mut self, level: log::Level) -> Result<Child>;

    /// Logs full command if at required level
    fn log(&mut self, level: log::Level) -> &mut Self;
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

    fn get_full_command(&self) -> String {
        format!(
            "{} {}",
            self.get_program().to_string_lossy(),
            self.get_args()
                .collect::<Vec<_>>()
                .join(std::ffi::OsStr::new(" "))
                .to_string_lossy(),
        )
    }

    fn log_output(&mut self, level: log::Level) -> std::io::Result<Output> {
        let output = self.output();
        match output.as_ref() {
            Ok(output) => log::log!(
                level,
                "Command {:?} (output)\n  STDOUT: {:?}\n  STDERR: {:?}\n  STATUS: {:?}",
                self.get_full_command(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stdout),
                output.status,
            ),
            Err(err) => log::log!(
                level,
                "Command {:?} (output)\n  ERROR: {:?}",
                self.get_full_command(),
                err,
            ),
        }

        output
    }

    fn log_output_anyhow(&mut self, level: log::Level) -> Result<Output> {
        match self.log_output(level) {
            Ok(output) if output.status.success() => Ok(output),
            Ok(output) => Err(anyhow!(
                "Command {:?} failed with code {:?}",
                self.get_full_command(),
                output.status
            )),
            Err(err) => {
                Err(err).with_context(|| anyhow!("Command {:?} failed", self.get_full_command()))
            }
        }
    }

    fn log_status(&mut self, level: log::Level) -> std::io::Result<ExitStatus> {
        let status = self.status();
        match status.as_ref() {
            Ok(status) => log::log!(
                level,
                "Command {:?} (status)\n  STATUS: {:?}",
                self.get_full_command(),
                status,
            ),
            Err(err) => log::log!(
                level,
                "Command {:?} (status)\n  ERROR {:?}",
                self.get_full_command(),
                err,
            ),
        }

        status
    }

    fn log_status_anyhow(&mut self, level: log::Level) -> Result<ExitStatus> {
        match self.log_status(level) {
            Ok(status) if status.success() => Ok(status),
            Ok(status) => Err(anyhow!(
                "Command {:?} failed with code {:?}",
                self.get_full_command(),
                status
            )),
            Err(err) => {
                Err(err).with_context(|| anyhow!("Command {:?} failed", self.get_full_command()))
            }
        }
    }

    fn log_spawn(&mut self, level: log::Level) -> std::io::Result<Child> {
        let child = self.spawn();
        match child.as_ref() {
            Ok(_) => log::log!(level, "Command {:?} (spawn)", self.get_full_command(),),
            Err(err) => log::log!(
                level,
                "Command {:?} (spawn)\n  ERROR {:?}",
                self.get_full_command(),
                err,
            ),
        }

        child
    }

    fn log_spawn_anyhow(&mut self, level: log::Level) -> Result<Child> {
        self.log_spawn(level)
            .with_context(|| anyhow!("Command {:?} failed", self.get_full_command()))
    }

    fn log(&mut self, level: log::Level) -> &mut Self {
        log::log!(level, "Command {:?}", self.get_full_command());

        self
    }
}
