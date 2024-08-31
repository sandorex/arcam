use crate::{ExitResult, cli};
use crate::util::command_extensions::*;
use crate::util::{self, Engine};

pub fn kill_container(engine: Engine, dry_run: bool, cli_args: &cli::CmdKillArgs) -> ExitResult {
    if ! util::is_box_container(&engine, &cli_args.container) {
        eprintln!("Container {} is not owned by box or does not exist", &cli_args.container);
        return Err(1);
    }

    // simple shitty prompt
    // if not yes then yes, but if yes then no yes
    if ! cli_args.yes {
        use std::io::Write;
        let mut s = String::new();

        print!("Are you sure you want to kill container {:?} ? [y/N] ", &cli_args.container);

        let _ = std::io::stdout().flush();

        std::io::stdin().read_line(&mut s).expect("Could not read stdin");
        s = s.trim().to_string();

        match s.to_lowercase().as_str() {
            "y"|"yes" => {},
            _ => return Err(1),
        }
    }

    let timeout = cli_args.timeout.to_string();

    let mut cmd = Command::new(&engine.path);
    cmd.args(["container", "stop", "--time", &timeout, &cli_args.container]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        cmd
            .status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}
