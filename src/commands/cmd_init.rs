//! Contains all code that should run inside the container as the init

use crate::util::command_extensions::*;
use crate::{cli, ExitResult, FULL_VERSION};
use std::fs::OpenOptions;
use std::{env, fs};
use std::os::unix::fs::{chown, lchown, symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::prelude::*;
use base64::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct InitArgs {
    pub on_init_pre: Vec<String>,
    pub on_init_post: Vec<String>,
}

impl InitArgs {
    pub fn decode(input: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let decoded = BASE64_STANDARD.decode(input)?;
        Ok(bson::from_slice(&decoded)?)
    }

    pub fn encode(&self) -> Result<String, Box<dyn std::error::Error>> {
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
fn initialization(_args: &InitArgs) -> ExitResult {
    println!("{} {}", env!("CARGO_BIN_NAME"), FULL_VERSION);

    let user = std::env::var("HOST_USER")
        .expect("HOST_USER is undefined");
    let uid = std::env::var("HOST_USER_UID")
        .expect("HOST_USER_UID is undefined");
    let gid = std::env::var("HOST_USER_GID")
        .expect("HOST_USER_GID is undefined");

    let uid_u: u32 = uid.parse().unwrap();
    let gid_u: u32 = gid.parse().unwrap();

    let home = format!("/home/{}", user);
    let shell = find_preferred_shell();

    // setting up the user
    let user_found = Command::new("getent")
        .args(["passwd", &user])
        .output()
        .expect("Error executing getent")
        .status.success();
    if !user_found {
        println!("Creating user {:?}", user);

        Command::new("useradd")
            .args([
                "--shell", shell,
                "--home-dir", &home,
                "--uid", &uid,
                "--gid", &gid,
                "--no-create-home",
                &user,
            ])
            .status()
            .expect("Error executing useradd")
            .to_exitcode()?;
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
            .to_exitcode()?;
    }

    println!("Setting up the user home");

    // create the home directory if missing
    if !Path::new(&home).exists() {
        fs::create_dir(&home)
            .expect("Failed to create user home");
    }

    // make sure its own by the user
    chown(&home, Some(uid_u), Some(gid_u))
        .expect("Failed to chown user home directory");

    let mut files: Vec<PathBuf> = vec![];
    let mut dirs: Vec<PathBuf> = vec![];

    // NOTE changing directory so i get './' paths and don't have to deal with path manipulation
    env::set_current_dir("/etc/skel").unwrap();

    walk_dir(Path::new("."), &mut files, &mut dirs);

    env::set_current_dir("/").unwrap();

    // recreate all the directories
    for dir in &dirs {
        let source = Path::new("/etc/skel").join(dir);
        let dest = Path::new(&home).join(dir);

        if !dest.exists() {
            fs::create_dir(&dest)
                .unwrap();
        }

        chown(&dest, Some(uid_u), Some(gid_u))
            .unwrap();

        clone_perm(&source, &dest)
            .unwrap();
    }

    // clone all the files including symlinks
    for file in &files {
        let source = Path::new("/etc/skel").join(file);
        let dest = Path::new(&home).join(file);

        // NOTE fs::copy fails on broken symlinks
        if source.is_symlink() {
            symlink(source.read_link().unwrap(), &dest)
                .unwrap();
        } else {
            // NOTE it seems copy clones permissions as well so its fine
            fs::copy(&source, &dest)
                .unwrap();
        }

        lchown(&dest, Some(uid_u), Some(gid_u))
            .unwrap();
    }

    // create the runtime dir whatever it is
    {
        let dest = std::env::var("XDG_RUNTIME_DIR").unwrap();

        // create all the dirs required
        fs::create_dir_all(&dest)
            .unwrap();

        // make sure user owns it
        chown(&dest, Some(uid_u), Some(gid_u))
            .unwrap();

        // set permission
        let mut perm = fs::symlink_metadata(&dest)
            .unwrap()
            .permissions();

        perm.set_mode(0o700);

        fs::set_permissions(&dest, perm)
            .unwrap();
    }

    let has_sudo = Path::new("/bin/sudo").exists();
    if has_sudo {
        println!("Enabling passwordless sudo for everyone");

        let mut file = OpenOptions::new()
            .append(true)
            .open("/etc/sudoers")
            .unwrap();

        // disable hostname resolving
        writeln!(file, "Defaults !fqdn")
            .unwrap();

        // allow everything without a password for everyone
        writeln!(file, "ALL ALL = (ALL) NOPASSWD: ALL")
            .unwrap();
    } else {
        println!("Sudo not found, enabling passwordless su");

        // just remove root password for passwordless su
        Command::new("passwd")
            .args(["-d", "root"])
            .status()
            .expect("Failed to execute passwd")
            .to_exitcode()?;
    }

    let init_dir = Path::new("/init.d");
    if init_dir.exists() {
        for entry in init_dir.read_dir().unwrap().flatten() {
            // make sure its executable
            if !entry.metadata().is_ok_and(|x| x.permissions().mode() & 0o111 != 0) {
                make_executable(&entry.path()).unwrap();
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
                    .status()
                    .unwrap()
            } else {
                Command::new("su")
                    .args([&user, "-c"])
                    .arg(entry.path())
                    .status()
                    .unwrap()
            };

            if ! cmd.success() {
                eprintln!("Script {:?} has failed with exit code {}", entry.path(), cmd.to_exitcode().unwrap_err());
            }
        }
    }

    // signalize that init is done
    fs::write("/initialized", "y")
        .unwrap();

    println!("Initialization finished");

    Ok(())
}

/// Gets PIDs of all root processes inside the container as they all share PPID of 0
fn get_root_processes() -> Result<Vec<String>, u8> {
    let cmd = Command::new("pgrep")
        .args(["-P", "0"])
        .output()
        .expect("Failed to execute pgrep");

    cmd.to_exitcode()?;

    let stdout = String::from_utf8_lossy(&cmd.stdout);

    let lines: Vec<String> = stdout
        .trim()
        .lines()
        .filter(|x| *x != "1") // this binary is is pid 1
        .map(|x| x.to_string())
        .collect();

    Ok(lines)
}

pub fn container_init(cli_args: cli::CmdInitArgs) -> ExitResult {
    // decode the encoded args
    let args = match InitArgs::decode(&cli_args.args) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Error decoding encoded args {:?}: {}", cli_args.args, err);
            return Err(1);
        }
    };

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("Termination signal received");

        // stop the loop
        r.store(false, Ordering::SeqCst);
    }).expect("Error while setting signal handler");

    if !args.on_init_pre.is_empty() {
        let path = Path::new("/init.d/00_on_init_pre.sh");

        // write the init commands to single file
        fs::write(path, "#!/bin/sh").unwrap();
        fs::write(path, args.on_init_pre.join("\n")).unwrap();

        make_executable(path).unwrap();
    }

    if !args.on_init_post.is_empty() {
        let path = Path::new("/init.d/99_on_init_post.sh");

        // write the init commands to single file
        fs::write(path, "#!/bin/sh").unwrap();
        fs::write(path, args.on_init_post.join("\n")).unwrap();

        make_executable(path).unwrap();
    }

    initialization(&args)?;

    // simply wait until container gets killed
    while running.load(Ordering::SeqCst) {
        // from my testing the delay from this does not really matter but an empty while generated
        // a high cpu usage which is not ideal in the slightest
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // find leftover processes
    let pids = get_root_processes()?;

    // do not run kill if there are no processes to kill
    if !pids.is_empty() {
        println!("Propagating signal to processes");

        // to avoid more crates just use kill command
        Command::new("kill")
            // be verbose
            // send TERM then KILL after 10s
            .args(["--verbose", "--timeout", "10000", "KILL", "--signal", "TERM"])
            .args(&pids)
            .status()
            .expect("Failed to execute kill")
            .to_exitcode()?;
    }

    println!("Goodbye!");

    Ok(())
}
