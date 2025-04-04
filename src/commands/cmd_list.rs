use crate::cli::CmdListArgs;
use crate::command_extensions::*;
use crate::prelude::*;

// TODO this could be reworked with inspect and actually getting the information
pub fn print_containers(ctx: Context, args: CmdListArgs) -> Result<()> {
    let mut cmd = ctx.engine.command();
    cmd.args([
        "container",
        "ls",
        "--filter",
        format!("label={}", crate::APP_NAME).as_str(),
        "--format",
        format!(
            "{{{{.Names}}}}\t{{{{.Image}}}}\t{{{{.Labels.{}}}}}\t{{{{.Ports}}}}",
            crate::CONTAINER_LABEL_HOST_DIR
        )
        .as_str(),
    ]);

    // filter the container by host_dir
    if args.here {
        cmd.arg("--filter");
        cmd.arg(format!(
            "label={}={}",
            crate::CONTAINER_LABEL_HOST_DIR,
            ctx.cwd.to_string_lossy()
        ));
    }

    if ctx.dry_run {
        cmd.log(log::Level::Error);
    } else if args.raw {
        // just run it raw
        cmd.log_status_anyhow(log::Level::Debug)?;
    } else {
        let output = cmd.log_output(log::Level::Debug)?;

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

            println!(
                "Container {:?} at {:?} {}",
                name,
                ws,
                if std::path::Path::new(ws) == ctx.cwd {
                    "*"
                } else {
                    " "
                }
            );
            println!("  image: {:?}", image);
            if !ports.is_empty() {
                println!("  ports: {}", ports);
            }
        }
    }

    Ok(())
}
