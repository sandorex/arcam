use crate::{Engine, ExitResult};
use crate::cli::cli_image::CmdImageBuildArgs;
use crate::util::command_extensions::*;
use std::process::Command;

fn gen_time_tag() -> String {
    // basically ISO8601 without special characters
    format!("{}", chrono::offset::Local::now().format("%Y%m%dT%H%M%S"))
}

pub fn build_image(engine: &Engine, dry_run: bool, cli_args: CmdImageBuildArgs) -> ExitResult {
    let tag: Option<String> = match &cli_args.tag {
        // if empty then autogenerate it
        Some(x) if x.is_empty() => Some(gen_time_tag()),

        // if provided then use that
        Some(x) => Some(x.clone()),

        // otherwise nothing
        None => None,
    };

    // use provided name, fallback to directory name
    let name: &str = &cli_args.name.unwrap_or_else(|| {
        std::env::current_dir()
            .expect("Could not get cwd")
            .file_name()
            .expect("Error cwd is a relative path")
            .to_string_lossy()
            .to_string()
    });

    // prefer the provided containerfile but default to either Containerfile or Dockerfile if they
    // exist
    let file: &str = {
        if let Some(x) = &cli_args.containerfile {
            x
        } else if std::path::Path::new("Containerfile").exists() {
            "Containerfile"
        } else if std::path::Path::new("Dockerfile").exists() {
            "Dockerfile"
        } else {
            eprintln!("Could not find Containerfile or Dockerfile");
            return Err(1);
        }
    };

    let mut cmd = Command::new(&engine.path);
    cmd.args([
        "build",
        "--security-opt", "label=disable",
        "--file", file,
    ]);

    if let Some(x) = &tag {
        // use name and tag
        cmd.args(["--tag", format!("{}:{}", name, x).as_str()]);
    } else {
        // use just name
        cmd.args(["--tag", name]);
    }

    if cli_args.no_cache {
        cmd.arg("--no-cache");
    }

    // mount dotfiles if provided
    if let Some(dotfiles) = &cli_args.dotfiles {
        cmd.args(["--volume", format!("/dotfiles:{}:ro,nocopy", dotfiles).as_str()]);
    }

    // if provided set context directory
    // NOTE podman allows '.' as the context directory
    cmd.arg(cli_args.build_dir.unwrap_or_else(|| ".".to_string()));

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        cmd.status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}
