use crate::cli::CmdStartArgs;
use crate::command_extensions::*;
use crate::prelude::*;
use crate::APP_NAME;
use std::collections::HashMap;
use std::path::Path;

/// Get hostname from system using `hostname` command
pub fn get_hostname() -> Result<String> {
    // try to get hostname from env var
    if let Ok(env_hostname) = std::env::var("HOSTNAME") {
        log::debug!("Getting hostname from environ");
        return Ok(env_hostname);
    }

    log::debug!("Getting hostname using hostname command");

    // then as a fallback use hostname executable
    let cmd = Command::new("hostname")
        .output()
        .with_context(|| "Could not call hostname")?;

    let hostname = String::from_utf8_lossy(&cmd.stdout);

    if !cmd.status.success() || hostname.is_empty() {
        return Err(anyhow!("Unable to get hostname from host"));
    }

    Ok(hostname.trim().into())
}

/// Generates random name using adjectives list
pub fn generate_name() -> String {
    const ADJECTIVES_ENGLISH: &str = include_str!("adjectives.txt");

    let adjectives: Vec<&str> = ADJECTIVES_ENGLISH.lines().collect();
    let adjective = adjectives
        .get(rand::random::<u64>() as usize % adjectives.len())
        .unwrap();

    // allow custom container suffix but default to bin name
    let suffix =
        std::env::var(crate::ENV_CONTAINER_SUFFIX).unwrap_or_else(|_| APP_NAME.to_string());

    format!("{}-{}", adjective, suffix)
}

/// Finds all terminfo directories on host so they can be mounted in the container so no terminfo
/// installing is required
///
/// This function is required as afaik only debian has non-standard paths for terminfo
pub fn find_terminfo() -> Vec<String> {
    let mut args: Vec<String> = vec![];

    log::debug!("Looking for terminfo directories on host system");

    let mut existing: Vec<String> = vec![];
    for x in ["/usr/share/terminfo", "/usr/lib/terminfo", "/etc/terminfo"] {
        if std::path::Path::new(x).exists() {
            log::debug!("Found {x:?}");
            args.push(format!("--volume={0}:/host{0}:ro", x));
            existing.push(x.into());
        }
    }

    let mut terminfo_env = "".to_string();

    // add first the host ones as they are preferred
    for x in &existing {
        terminfo_env.push_str(format!("/host{}:", x).as_str());
    }

    // add container ones as fallback
    for x in &existing {
        terminfo_env.push_str(format!("{}:", x).as_str());
    }

    // remove leading ':'
    if terminfo_env.chars().last().unwrap_or(' ') == ':' {
        terminfo_env.pop();
    }

    // generate the env variable to find them all
    args.push(format!("--env=TERMINFO_DIRS={}", terminfo_env));

    args
}

pub fn resolve_capabilities(cli_args: &CmdStartArgs, cmd: &mut Command) {
    // NOTE podman does not support drop and add at the same time, once dropped its dropped so i
    // want to extend that so config can overwrite it and then cli can overwrite the overwritten
    {
        // list of all capabilities mentioned, true => add, false => drop
        let mut caps = HashMap::<&str, bool>::new();

        for i in &cli_args.capabilities {
            match i.strip_prefix("!") {
                Some(x) => caps.insert(x, false),
                None => caps.insert(i, true),
            };
        }

        for (cap, val) in caps {
            if val {
                cmd.arg(format!("--cap-add={}", cap));
            } else {
                cmd.arg(format!("--cap-drop={}", cap));
            }
        }
    }
}

pub fn mount_wayland(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    // try to pass through wayland socket
    if cli_args.wayland.unwrap_or(false) {
        // prefer ARCAM_WAYLAND_DISPLAY
        if let Ok(wayland_display) =
            std::env::var(crate::ENV_WAYLAND_DISPLAY).or(std::env::var("WAYLAND_DISPLAY"))
        {
            let socket_path = format!("/run/user/{}/{}", ctx.user_id, wayland_display);
            if Path::new(&socket_path).exists() {
                log::debug!("Found wayland socket at {socket_path:?}");

                // TODO pass XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
                cmd.args([
                    format!("--volume={0}:{0}", socket_path),
                    format!("--env=WAYLAND_DISPLAY={}", wayland_display),
                ]);
            } else {
                return Err(anyhow!(
                    "Could not find the wayland socket {:?}",
                    socket_path
                ));
            }

            // add fonts just in case
            cmd.arg("--volume=/usr/share/fonts:/usr/share/fonts/host:ro");

            // legacy ~/.fonts
            let home_dot_fonts = ctx.user_home.join(".fonts");
            if home_dot_fonts.exists() {
                cmd.arg(format!(
                    "--volume={}:/usr/share/fonts/host_dot:ro",
                    home_dot_fonts.to_string_lossy()
                ));
            }

            // font dir ~/.local/share/fonts
            let home_dot_local_fonts = ctx.user_home.join(".local").join("share").join("fonts");

            if home_dot_local_fonts.exists() {
                cmd.arg(format!(
                    "--volume={}:/usr/share/fonts/host_local:ro",
                    home_dot_local_fonts.to_string_lossy()
                ));
            }
        } else {
            return Err(anyhow!(
                "Could not pass through wayland socket as WAYLAND_DISPLAY is not defined"
            ));
        }
    }

    Ok(())
}

pub fn mount_additional_mounts(
    ws_dir: &Path,
    cli_args: &CmdStartArgs,
    cmd: &mut Command,
) -> Result<()> {
    for m in &cli_args.mount {
        let mount = Path::new(m);
        if mount.exists() {
            if !mount.is_dir() {
                return Err(anyhow!("Mountpoint {:?} is not a directory", mount));
            }

            // get the absolute path
            let mount = mount.canonicalize().unwrap();

            log::debug!("Mounting additional mount {mount:?}");

            cmd.arg(format!(
                "--volume={}:{}/{}",
                mount.to_string_lossy(),
                ws_dir.to_string_lossy(),
                mount.file_name().unwrap().to_string_lossy()
            ));
        } else {
            return Err(anyhow!("Mountpoint {:?} does not exist", mount));
        }
    }

    Ok(())
}

pub fn mount_audio(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    // try to pass audio
    if cli_args.audio.unwrap_or(false) {
        // TODO see if passing pipewire or alsa is possible too
        let socket_path = format!("/run/user/{}/pulse/native", ctx.user_id);
        if Path::new(&socket_path).exists() {
            cmd.args([
                format!("--volume={0}:{0}", socket_path),
                format!("--env=PULSE_SERVER=unix:{}", socket_path),
            ]);

            log::debug!("Pulseaudio socket found at {socket_path:?}");
        } else {
            return Err(anyhow!(
                "Could not find pulseaudio socket to pass to the container"
            ));
        }
    }

    Ok(())
}

pub fn mount_ssh_agent(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    if cli_args.ssh_agent.unwrap_or(false) {
        if let Ok(ssh_sock) = std::env::var("SSH_AUTH_SOCK") {
            if Path::new(&ssh_sock).exists() {
                cmd.args([
                    format!("--volume={}:/run/user/{}/ssh-auth", ssh_sock, ctx.user_id),
                    format!("--env=SSH_AUTH_SOCK=/run/user/{}/ssh-auth", ctx.user_id),
                ]);

                log::debug!("ssh-agent socket found at {ssh_sock:?}");
            } else {
                return Err(anyhow!(
                    "Socket does not exist at {:?} (ssh-agent)",
                    ssh_sock
                ));
            }
        } else {
            return Err(anyhow!(
                "Could not pass through ssh-agent as SSH_AUTH_SOCK is not defined"
            ));
        }
    }

    Ok(())
}

pub fn mount_session_bus(ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    if cli_args.session_bus.unwrap_or(false) {
        if let Ok(dbus_addr) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            if let Some(dbus_sock) = dbus_addr.strip_prefix("unix:path=") {
                if Path::new(&dbus_sock).exists() {
                    cmd.args([
                        format!("--volume={}:/run/user/{}/bus", dbus_sock, ctx.user_id),
                        format!(
                            "--env=DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus",
                            ctx.user_id
                        ),
                    ]);

                    log::debug!("Session dbus socket found at {dbus_sock:?}");
                } else {
                    return Err(anyhow!(
                        "Socket does not exist at {:?} (session bus)",
                        dbus_sock
                    ));
                }
            } else {
                return Err(anyhow!(
                    "Invalid format for DBUS_SESSION_BUS_ADDRESS={:?}",
                    dbus_addr
                ));
            }
        } else {
            return Err(anyhow!(
                "Could not pass through session bus as DBUS_SESSION_BUS_ADDRESS is not defined"
            ));
        }
    }

    Ok(())
}

/// Writes text to file inside the container
pub fn write_to_file(ctx: &Context, container: &str, file: &Path, content: &str) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    log::trace!("Writing data to file {file:?}");

    // write to file using tee
    #[allow(clippy::zombie_processes)]
    let mut child = ctx
        .engine
        .command()
        .args([
            "exec",
            "-i",
            "--user",
            "root",
            container,
            "tee",
            &file.to_string_lossy(),
        ])
        .stdin(Stdio::piped()) // pipe into stdin but ignore stdout/stderr
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .log_spawn(log::Level::Debug)
        .expect(crate::ENGINE_ERR_MSG);

    let mut stdin = child
        .stdin
        .take()
        .with_context(|| anyhow!("Failed to open child stdin"))?;

    stdin.write_all(content.as_bytes())?;

    // NOTE drop is important here otherwise stdin wont close
    drop(stdin);

    let result = child.wait()?;

    if result.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "Error writing to file {:?} in container ({})",
            file,
            result.get_code()
        ))
    }
}
