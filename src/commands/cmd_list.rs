use crate::util::{Engine, command_extensions::*};
use crate::ExitResult;
use crate::cli::CmdListArgs;

pub fn print_containers(engine: Engine, dry_run: bool, args: CmdListArgs) -> ExitResult {
    let mut cmd = Command::new(&engine.path);
    cmd.args([
        "container", "ls",
        "--filter", format!("label={}", crate::APP_NAME).as_str(),
        // TODO host_dir label should be a global const!
        "--format", "{{.Names}}\t{{.Image}}\t{{.Labels.host_dir}}\t{{.Ports}}"
    ]);

    // filter the container by host_dir
    if args.here {
        cmd.arg("--filter");
        cmd.arg(format!("label=host_dir={}", std::env::current_dir()
            .expect("Error getting cwd")
            .to_string_lossy()));
    }

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        if args.raw {
            // just run it raw
            cmd.status()
                .expect(crate::ENGINE_ERR_MSG)
                .to_exitcode()
        } else {
            let output = cmd.output()
                .expect(crate::ENGINE_ERR_MSG);

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for (index, line) in stdout.lines().enumerate() {
                    let columns: Vec<&str> = line.trim_start().split("\t").collect();

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
}
