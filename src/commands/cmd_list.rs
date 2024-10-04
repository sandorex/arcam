use crate::util::command_extensions::*;
use crate::{util::Engine, ExitResult};

pub fn print_containers(engine: Engine, dry_run: bool) -> ExitResult {
    let mut cmd = Command::new(&engine.path);
    cmd.args([
        "container", "ls",
        "--filter", format!("label={}", crate::APP_NAME).as_str(),
        "--format", "{{.Names}}|{{.Image}}|{{.Labels.host_dir}}|{{.Ports}}"
    ]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        let output = cmd.output()
            .expect(crate::ENGINE_ERR_MSG);

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for (index, line) in stdout.lines().enumerate() {
                let columns: Vec<&str> = line.trim().split("|").collect();

                let name = columns[0];
                let image = columns[1];
                let ws = columns[2];
                let ports = columns[3];

                // format nicely by adding a newline
                if index != 0 {
                    println!();
                }

                println!("Container {:?} at {}", name, ws);
                println!("  image: {}", image);
                if !ports.is_empty() {
                    println!("  ports: {}", ports);
                }
            }

            Ok(())
        } else {
            output.to_exitcode()
        }
    }
}
