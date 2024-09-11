use crate::util::command_extensions::*;
use crate::{util::Engine, ExitResult};

pub fn print_containers(engine: Engine, dry_run: bool) -> ExitResult {
    let mut cmd = Command::new(&engine.path);
    cmd.args([
        "container", "ls",
        "--filter", format!("label={}", crate::BIN_NAME).as_str(),
        "--format", "{{.Names}}|{{.Image}}|{{.Labels.host_dir}}|{{.Ports}}"
    ]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        let output = cmd.output()
            .expect(crate::ENGINE_ERR_MSG);

        if output.status.success() {
            println!(
                "{0: <14} {1: <40} {2: <35} {3: <40}",
                "NAMES", "IMAGE", "WS", "PORTS"
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            for i in stdout.lines() {
                let columns: Vec<&str> = i.trim().split("|").collect();

                println!(
                    "{0: <14} {1: <40} {2: <40} {3: <30}",
                    columns[0], columns[1], columns[2], columns[3],
                );
            }

            Ok(())
        } else {
            output.to_exitcode()
        }
    }
}
