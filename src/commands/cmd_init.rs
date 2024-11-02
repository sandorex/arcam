//! Contains all code that should run inside the container as the init

use crate::util::command_extensions::*;
use crate::{cli, FULL_VERSION};
use crate::prelude::*;
use std::fs::OpenOptions;
use std::{env, fs};
use std::os::unix::fs::{chown, lchown, symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::prelude::*;
use base64::prelude::*;

/// This file existing is a signal when container initialization is finished
pub const INITALIZED_FLAG_FILE: &str = "/initialized";

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct InitArgs {
    pub on_init_pre: Vec<String>,
    pub on_init_post: Vec<String>,
    pub automatic_idle_shutdown: bool,
}

impl InitArgs {
    pub fn decode(input: &str) -> Result<Self> {
        let decoded = BASE64_STANDARD.decode(input)?;
        Ok(bson::from_slice(&decoded)?)
    }

    pub fn encode(&self) -> Result<String> {
        Ok(BASE64_STANDARD.encode(bson::to_vec(self)?))
    }
}

fn find_preferred_shell() -> &'static str {
    const SHELLS: [&str; 3] = [
        "/bin/fish",
        "/bin/zsh",
        "/bin/bash",
    ];

    for shell in SHELLS {
        if Path::new(shell).exists() {
            return shell;
        }
    }

    // use sh as fallback
    "/bin/sh"
}

/// Walk recursively collecting files/symlinks into one vec, dirs into another
fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>, dirs: &mut Vec<PathBuf>) {
    let iter = fs::read_dir(dir).unwrap();
    for entry in iter {
        match entry {
            Ok(x) => match x.file_type() {
                Ok(t) => {
                    // strip prefix
                    let clean_path = x.path().strip_prefix("./").unwrap().to_path_buf();

                    if t.is_dir() {
                        dirs.push(clean_path.to_path_buf());
                        walk_dir(x.path().as_path(), files, dirs);
                    } else if t.is_file() || t.is_symlink() {
                        // save paths of files and symlinks
                        files.push(clean_path.to_path_buf());
                    } else {
                        // all other files are errors cause they should not be there
                        eprintln!("Invalid file type at {:?}", clean_path);
                    }
                },
                Err(x) => eprintln!("Could not determine file type: {}", x),
            },
            Err(x) => eprintln!("Error while reading directory: {}", x),
        }
    }
}

/// Clone permissions between two paths, specificially `mode`
fn clone_perm(source: &Path, dest: &Path) -> Result<(), std::io::Error> {
    let source_perm = source.symlink_metadata()?.permissions();
    let mut dest_perm = dest.symlink_metadata()?.permissions();

    dest_perm.set_mode(source_perm.mode());

    fs::set_permissions(dest, dest_perm)?;

    Ok(())
}

/// Basically does same thing as `chmod +x`
fn make_executable(path: &Path) -> Result<(), std::io::Error> {
    let mut perm = path.metadata()?.permissions();

    // make file executable
    perm.set_mode(perm.mode() | 0o111);

    fs::set_permissions(path, perm)?;

    Ok(())
}

// TODO create /tmp/.X11-unix just so its properly owned by root? and has correct permissions?
fn initialization(_args: &InitArgs) -> Result<()> {
    println!("{} {}", env!("CARGO_BIN_NAME"), FULL_VERSION);

    let user = std::env::var("HOST_USER")
        .context("HOST_USER is undefined")?;
    let uid = std::env::var("HOST_USER_UID")
        .context("HOST_USER_UID is undefined")?;
    let gid = std::env::var("HOST_USER_GID")
        .context("HOST_USER_GID is undefined")?;

    let uid_u: u32 = uid.parse().unwrap();
    let gid_u: u32 = gid.parse().unwrap();

    let home = format!("/home/{}", user);
    let shell = find_preferred_shell();

    // setting up the user
    let user_found = Command::new("getent")
        .args(["passwd", &user])
        .output()
        .expect("Error executing getent")
        .status
        .success();

    let cmd = if !user_found {
        println!("Creating user {:?}", user);

        Command::new("useradd")
            .args([
                "--shell", shell,
                "--home-dir", &home,
                "--uid", &uid,
                "--user-group",
                "--no-create-home",
                &user,
            ])
            .status()
            .expect("Error executing useradd")
    } else {
        println!("Modifying user {:?}", user);

        Command::new("usermod")
            .args([
                "--home", &home,
                "--shell", shell,
                &user,
            ])
            .status()
            .expect("Error executing usermod")
    };

    if !cmd.success() {
        return Err(anyhow!("Error while setting up the user"));
    }

    println!("Setting up the user home");

    // create the home directory if missing
    if !Path::new(&home).exists() {
        fs::create_dir(&home)
            .context("Failed to create user home")?;
    }

    // make sure its own by the user
    chown(&home, Some(uid_u), Some(gid_u))
        .context("Failed to chown user home directory")?;

    // generate font cache just in case
    {
        println!("Recreating font cache");

        let cmd = Command::new("fc-cache")
            .status();

        match cmd {
            Ok(x) => if !x.success() {
                return Err(anyhow!("Failed to regenerate font cache"));
            },
            // some images may not have it so im just gonna ignore it
            Err(_) => println!("Failed to execute fc-cache, ignoring error.."),
        }
    }

    let mut files: Vec<PathBuf> = vec![];
    let mut dirs: Vec<PathBuf> = vec![];

    // NOTE changing directory so i get './' paths and don't have to deal with path manipulation
    env::set_current_dir("/etc/skel")?;

    walk_dir(Path::new("."), &mut files, &mut dirs);

    env::set_current_dir("/")?;

    // recreate all the directories
    for dir in &dirs {
        let source = Path::new("/etc/skel").join(dir);
        let dest = Path::new(&home).join(dir);

        if !dest.exists() {
            fs::create_dir(&dest)?;
        }

        chown(&dest, Some(uid_u), Some(gid_u))?;

        clone_perm(&source, &dest)?;
    }

    // clone all the files including symlinks
    for file in &files {
        let source = Path::new("/etc/skel").join(file);
        let dest = Path::new(&home).join(file);

        // NOTE fs::copy fails on broken symlinks
        if source.is_symlink() {
            symlink(source.read_link().unwrap(), &dest)?;
        } else {
            // NOTE it seems copy clones permissions as well so its fine
            fs::copy(&source, &dest)?;
        }

        lchown(&dest, Some(uid_u), Some(gid_u))?;
    }

    // create the runtime dir whatever it is
    {
        let dest = std::env::var("XDG_RUNTIME_DIR")?;

        // create all the dirs required
        fs::create_dir_all(&dest)?;

        // make sure user owns it
        chown(&dest, Some(uid_u), Some(gid_u))?;

        // set permission
        let mut perm = fs::symlink_metadata(&dest)?
            .permissions();

        perm.set_mode(0o700);

        fs::set_permissions(&dest, perm)?
    }

    let has_sudo = Path::new("/bin/sudo").exists();
    if has_sudo {
        println!("Enabling passwordless sudo for everyone");

        let mut file = OpenOptions::new()
            .append(true)
            .open("/etc/sudoers")?;

        // disable hostname resolving
        writeln!(file, "Defaults !fqdn")?;

        // allow everything without a password for everyone
        writeln!(file, "ALL ALL = (ALL) NOPASSWD: ALL")?;
    } else {
        println!("Sudo not found, enabling passwordless su");

        // just remove root password for passwordless su
        let code = Command::new("passwd")
            .args(["-d", "root"])
            .status()
            .expect("Failed to execute passwd")
            .get_code();

        if code != 0 {
            return Err(anyhow!("Error while setting passwd for root ({})", code));
        }
    }

    let init_dir = Path::new("/init.d");
    if init_dir.exists() {
        for entry in init_dir.read_dir().unwrap().flatten() {
            // make sure its executable
            if !entry.metadata().is_ok_and(|x| x.permissions().mode() & 0o111 != 0) {
                make_executable(&entry.path())?;
            }

            // accept both files and symlinks
            if !entry.file_type().is_ok_and(|x| x.is_file() || x.is_symlink()) {
                continue;
            }

            println!("Executing script {:?}", entry.path());

            // use sudo if available
            let cmd = if has_sudo {
                Command::new("sudo")
                    .args(["-u", &user])
                    .arg(entry.path())
                    .status()?
            } else {
                Command::new("su")
                    .args([&user, "-c"])
                    .arg(entry.path())
                    .status()?
            };

            if !cmd.success() {
                eprintln!("Script {:?} has failed with exit code {}", entry.path(), cmd.get_code());
            }
        }
    }

    // signalize that init is done
    fs::write(INITALIZED_FLAG_FILE, "y")
        .unwrap();

    println!("Initialization finished");

    Ok(())
}

/// Parse PID and PPID from contents of `/proc/PID/stat` file
fn parse_proc_stat(input: &str) -> (u64, u64) {
    // FORMAT: PID (EXE_NAME) STATE PPID ...
    // WARNING EXE_NAME can contain spaces, newlines basically anything

    let pid = input.split_once(" ")
        .unwrap()
        .0
        .parse::<u64>()
        .unwrap();

    // parse pid and ppid
    let ppid = {
        // remove everything before the executable name, cause its hard to parse
        let rest = input.rsplit_once(") ").unwrap().1;

        // split again and get the PPID
        rest.split(" ")
            .nth(1)
            .unwrap()
            .parse::<u64>()
            .unwrap()
    };

    (pid, ppid)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_proc_stat() {
        use super::parse_proc_stat;

        // names to test the algorithm with
        const NAMES: [&str; 4] = [
            "arcam",
            "((dw)",
            ")) ) ) ",
            r#") ;
_)
;w
dw
"#
        ];

        for name in NAMES {
            assert_eq!(
                parse_proc_stat(&format!(
                    "{} ({}) S {} 1 1 0 -1 4194560 219 8424 0 3 1 8 7 7 20 0 2 0 87254147 75227136 893 18446744073709551615 94609717866496 94609719346301 140731732879504 0 0 0 0 4096 17475 0 0 0 17 11 0 0 0 0 0 94609719712240 94609719767160 94610004168704 140731732880722 140731732880935 140731732880935 140731732881393 0",
                    69, name, 420
                )),
                (69, 420)
            );
        }
    }
}

/// Get PIDs of all root processes (pid 1 is excluded)
fn get_root_processes() -> Result<Vec<u64>> {
    let mut root_pids: Vec<u64> = vec![];

    // as container works bit weirdly each command ran from exec will have PPID
    // of 0, so im manually filtering cause pgrep is not always available
    for entry in fs::read_dir("/proc/")?.flatten() {
        // skip non numeric filenames
        if !entry.file_name().to_string_lossy().chars().all(|x| x.is_ascii_digit()) {
            continue
        }

        // skip non-dir entries
        match entry.file_type() {
            Ok(x) if x.is_dir() => {},
            _ => continue,
        }

        let (pid, ppid) = {
            // read the /proc/PID/stat
            let stat_file = fs::read_to_string(entry.path().join("stat"))?;

            parse_proc_stat(&stat_file)
        };

        // ppid of 0 means its root process but ignore the init binary itself
        if ppid == 0 && pid != 1 {
            root_pids.push(pid);
        }
    }

    Ok(root_pids)
}

pub fn container_init(cli_args: cli::CmdInitArgs) -> Result<()> {
    // decode the encoded args
    let args = match InitArgs::decode(&cli_args.args) {
        Ok(x) => x,
        Err(err) => {
            return Err(anyhow!("Error decoding encoded args {:?}: {}", cli_args.args, err));
        }
    };

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("Termination signal received");

        // stop the loop
        r.store(false, Ordering::SeqCst);
    }).expect("Error while setting signal handler");

    // create the dir always
    if !Path::new("/init.d").exists() {
        std::fs::create_dir("/init.d").unwrap();
    }

    if !args.on_init_pre.is_empty() {
        let path = Path::new("/init.d/00_on_init_pre.sh");

        // write the init commands to single file
        fs::write(path, format!("#!/bin/sh\n{}",
            args.on_init_pre.join("\n"))).unwrap();

        make_executable(path).unwrap();
    }

    if !args.on_init_post.is_empty() {
        let path = Path::new("/init.d/99_on_init_post.sh");

        // write the init commands to single file
        fs::write(path, format!("#!/bin/sh\n{}",
            args.on_init_post.join("\n"))).unwrap();

        make_executable(path).unwrap();
    }

    initialization(&args)?;

    // start thread to kill container if idle
    if args.automatic_idle_shutdown {
        let r = running.clone();

        std::thread::spawn(move || {
            while r.load(Ordering::SeqCst) {
                // check every 10 minutes
                std::thread::sleep(std::time::Duration::from_secs(30)); // temp 10s
                // std::thread::sleep(std::time::Duration::from_secs(10 * 60));

                let root_processes: Vec<u64> = get_root_processes()
                    .unwrap();

                // if there are not procesess then stop
                if root_processes.len() == 0 {
                    println!("Automatic shutdown commencing, no processes running");
                    r.store(false, Ordering::SeqCst);
                    break
                }
            }
        });
    }

    // simply wait until container gets killed
    while running.load(Ordering::SeqCst) {
        // from my testing the delay from this does not really matter but an
        // empty while loop generated a high cpu usage which is not ideal
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // find leftover processes
    // convert to string as command does not allow number arguments
    let pids = get_root_processes()?
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>();

    // TODO send the signal manually so there is no relience on kill command?
    // do not run kill if there are no processes to kill
    if !pids.is_empty() {
        println!("Propagating signal to processes");

        // to avoid more crates just use kill command
        let _ = Command::new("kill")
            // be verbose
            // send TERM then KILL after 10s
            .args(["--verbose", "--timeout", "10000", "KILL", "--signal", "TERM"])
            .args(&pids)
            .status()
            .expect("Failed to execute kill");
    }

    println!("Goodbye!");

    Ok(())
}
