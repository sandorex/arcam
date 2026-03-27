use crate::cli::CmdStartArgs;
use crate::command_extensions::*;
use crate::prelude::*;
use crate::APP_NAME;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

/// Get hostname from system using `hostname` command
pub fn get_hostname() -> Result<String> {
    // try to get hostname from env var
    if let Ok(env_hostname) = std::env::var("HOSTNAME") {
        log::debug!("Getting hostname from environment");
        return Ok(env_hostname);
    }

    log::debug!("Getting hostname using hostname command");

    // then as a fallback use hostname executable
    let cmd = Command::new("hostname")
        .log_output()
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

    // allow removing suffix by setting it to empty string
    if suffix.is_empty() {
        adjective.to_string()
    } else {
        format!("{}-{}", adjective, suffix)
    }
}

/// Finds all terminfo directories on host so they can be mounted in the container so no terminfo
/// installation is required
//
// Special cases:
// - Debian has non-standard paths for terminfo
//
// - NixOS uses TERMINFO_DIRS to set proper paths but.. the directories inside are all symlinks..
//   so i resorted to using `infocmp -D` instead
pub fn find_terminfo() -> Vec<String> {
    const STANDARD_PATHS: &[&str] = &["/usr/share/terminfo", "/usr/lib/terminfo", "/etc/terminfo"];

    let mut args: Vec<String> = vec![];
    let mut terminfo_env = "".to_string();
    let mut dirs: Vec<String> = vec![];

    // get terminfo directories using infocmp (experimental)
    match Command::new("infocmp").arg("-D").log_output_anyhow() {
        Ok(x) => {
            let stdout = String::from_utf8_lossy(&x.stdout);

            for dir in stdout.lines() {
                if std::fs::exists(dir).ok().unwrap_or(false) {
                    dirs.push(dir.to_string());
                }
            }
        }
        Err(err) => log::error!("Error getting terminfo directories using infocmp: {err}"),
    }

    // use TERMINFO_DIRS if defined
    if let Ok(env_dirs) = std::env::var("TERMINFO_DIRS") {
        log::debug!("Looking for terminfo directories from environment variable");

        // filter existing directories
        for dir in env_dirs.split(':') {
            if std::fs::exists(dir).ok().unwrap_or(false) {
                // do not add duplicates
                let dir = dir.to_string();
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }
    }

    log::debug!("Looking for standard terminfo directories");

    // find the standard paths
    for dir in STANDARD_PATHS {
        if std::fs::exists(dir).ok().unwrap_or(false) {
            let dir = dir.to_string();
            if dirs.contains(&dir) {
                dirs.push(dir.to_string());
            }
        }
    }

    // this certainly should not happen but just in case
    assert!(!dirs.is_empty(), "Could not find any TERMINFO directories!");

    log::debug!("Terminfo directories found:");
    for dir in &dirs {
        use rand::distr::{Alphanumeric, SampleString};

        let mountpoint = format!(
            "/host/terminfo/{}",
            // generate random name to prevent huge paths
            Alphanumeric.sample_string(&mut rand::rng(), 4)
        );
        log::debug!("{dir:?} -> {mountpoint:?}");

        args.push(format!("--volume={dir}:{mountpoint}:ro"));
        terminfo_env.push_str(format!("{mountpoint}:").as_str());
    }

    // add container paths to TERMINFO_DIRS
    for x in STANDARD_PATHS {
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

            // add fonts if they exist
            let system_fonts = Path::new("/usr/share/fonts");
            if system_fonts.exists() {
                cmd.arg(format!(
                    "--volume={}:/usr/share/fonts/host:ro",
                    system_fonts.to_string_lossy()
                ));
            }

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

pub fn gpu_passthrough(_ctx: &Context, cli_args: &CmdStartArgs, cmd: &mut Command) -> Result<()> {
    use std::fs;

    let gpus = cli_args.gpus.clone().unwrap_or_else(|| vec![]);

    // no gpus selected do nothing
    if gpus.is_empty() {
        return Ok(());
    }

    let mut files: Vec<String> = vec![];

    // 0 means copy all
    if gpus.contains(&0) {
        log::debug!("Adding all GPU devices");

        for entry in std::fs::read_dir("/dev/dri").expect("Error reading /dev/dri") {
            let entry = entry?;

            // just filter paths by name
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with("card") || filename.starts_with("renderD") {
                log::debug!("Found {:?}", entry.path());
                files.push(entry.path().to_string_lossy().to_string());
            }
        }
    } else {
        log::debug!("Adding GPU devices");

        for gpu_index in gpus {
            // NOTE: for some reason first card is /dev/dri/card1 and /dev/renderD128
            let card = format!("/dev/dri/card{gpu_index}");
            let render = format!("/dev/dri/renderD{}", 127 + gpu_index);

            if fs::exists(&card).is_ok_and(|x| x) && fs::exists(&render).is_ok_and(|x| x) {
                log::debug!("Found GPU {gpu_index} at {card:?} and {render:?}");

                files.push(card);
                files.push(render);
            } else {
                return Err(anyhow!("Could not find card with index {gpu_index}"));
            }
        }
    }

    for path in files {
        cmd.args([format!("--device={path}")]);
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
    // try to pass pipewire
    if cli_args.pipewire.unwrap_or(false) {
        let container_path = format!("/run/user/{}/pipewire-0", ctx.user_id);

        // respect PIPEWIRE_REMOTE if defined
        let host_path = match std::env::var("PIPEWIRE_REMOTE") {
            Ok(x) => x,
            Err(_) => container_path.clone(),
        };

        if Path::new(&host_path).exists() {
            cmd.args([
                format!("--volume={}:{}", host_path, container_path),
                format!("--env=PIPEWIRE_REMOTE={}", container_path),
            ]);

            log::debug!("pipewire socket found at {host_path:?}");
        } else {
            return Err(anyhow!("Could not find pipewire socket at {host_path:?}"));
        }
    }

    // try to pass pulseaudio
    if cli_args.pulseaudio.unwrap_or(false) {
        let container_path = format!("/run/user/{}/pulse/native", ctx.user_id);

        // respect PULSE_SERVER if defined
        let host_path = match std::env::var("PULSE_SERVER") {
            Ok(pulse_server) => {
                // only accept sockets i do not know if there are other protocols
                if let Some(path) = pulse_server.strip_prefix("unix:") {
                    path.to_string()
                } else {
                    return Err(anyhow!("Invalid PULSE_SERVER value {pulse_server:?}"));
                }
            }
            // fallback to the default
            Err(_) => container_path.clone(),
        };

        if Path::new(&host_path).exists() {
            cmd.args([
                format!("--volume={}:{}", host_path, container_path),
                format!("--env=PULSE_SERVER=unix:{}", container_path),
            ]);

            log::debug!("pulseaudio socket found at {host_path:?}");
        } else {
            return Err(anyhow!("Could not find pulseaudio socket at {host_path:?}"));
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
        .log_spawn()
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

/// Expands vars from map using bash-like syntax `$HOME` and `${HOME}`, also supports default and
/// error suffixes like in bash `${HOME:?}` (error if undefined) and `${HOME:-default}` where
/// "default" will be the value if undefined
pub fn expand_vars<F>(input: &str, mut getter: F) -> Result<String>
    where F: FnMut(&str) -> Option<String>
{
    // compile regex only once
    static RE: LazyLock<regex::Regex> = LazyLock::new(||
        regex::Regex::new(r"\$\{([^}]*)\}|\$([A-Za-z_][A-Za-z0-9_]*\b|\$)"
    ).unwrap());

    let mut output = String::with_capacity(input.len() + 256);
    let mut last_range_end = 0usize;

    for m in RE.captures_iter(input) {
        let full = m.get(0).unwrap();
        let curly = m.get(1).is_some();

        let name = m.get(1).unwrap_or_else(|| m.get(2).unwrap()).as_str();
        let (name, default): (&str, Option<&str>) = if curly {
            if name.ends_with(":?") {
                // no default value error
                (&name[..name.len() - 2], None)
            } else if name.contains(":-") {
                let (name, default) = name.split_once(":-").unwrap();

                // use specified default value
                (name, Some(default))
            } else {
                // by default subtitute with empty string
                (name, Some(""))
            }
        } else if name == "$" {
            // escape the sign by just doing '$$'
            ("", Some("$"))
        } else {
            (name, Some(""))
        };

        // if name is empty then just use default
        let value = if name.is_empty() {
            default.map(|x| x.to_owned())
        } else {
            getter(name)
                .or_else(|| default.map(|x| x.to_owned()))
        };

        // if still None then just error out
        let Some(value) = value else {
            return Err(anyhow!("Error env variable {name:?} is undefined"));
        };

        // push left part of the input
        output.push_str(&input[last_range_end..full.start()]);

        // push the replacement
        output.push_str(&value);

        // save the end position
        last_range_end = full.end();
    }

    // push the rest of the string
    output.push_str(&input[last_range_end..]);

    Ok(output)
}

/// Wrapper around `expand_vars` to use a map instead of a function
#[allow(unused)]
pub fn expand_vars_map(input: &str, values: &HashMap<String, String>) -> Result<String> {
    expand_vars(input, |x| {
        values.get(x).cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_vars() {
        assert_eq!(
            expand_vars_map(
                "One !#F# $# @{  } $one ${two} ${three:-} $four ${four:?} five",
                &HashMap::from([
                    ("one".to_owned(), "1".to_owned()),
                    ("two".to_owned(), "2".to_owned()),
                    ("three".to_owned(), "3".to_owned()),
                    ("four".to_owned(), "4".to_owned()),
                ])
            ).unwrap(),
            "One !#F# $# @{  } 1 2 3 4 4 five".to_owned()
        );

        // make sure by default it expands to nothing
        assert_eq!(
            expand_vars_map(
                "$one ${one} ${one:-2}",
                &HashMap::from([])
            ).unwrap(),
            "  2".to_owned()
        );

        // make sure it errors out if empty
        assert!(expand_vars_map(
            "${one:?}",
            &HashMap::from([])
        ).is_err());

        assert_eq!(
            expand_vars_map(
                "/${one:?}/",
                &HashMap::from([
                    ("one".to_owned(), "1".to_owned())
                ])
            ).unwrap(),
            "/1/".to_owned()
        );

        assert_eq!(
            expand_vars_map(
                "cd $HOME/.config",
                &HashMap::from([
                    ("HOME".to_owned(), "/home/user".to_owned())
                ])
            ).unwrap(),
            "cd /home/user/.config".to_owned()
        );

        assert_eq!(
            expand_vars_map(
                "cd $HOME/.config/${CFG_DIR:-arcam}/config.toml",
                &HashMap::from([
                    ("HOME".to_owned(), "/home/user".to_owned())
                ])
            ).unwrap(),
            "cd /home/user/.config/arcam/config.toml".to_owned()
        );

        assert_eq!(
            expand_vars_map(
                "/$${x}/$$xx",
                &HashMap::from([])
            ).unwrap(),
            "/${x}/$xx".to_owned()
        );
    }
}
