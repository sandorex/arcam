//! Contains all code that should run inside the container as the init

use crate::util::command_extensions::*;
use crate::{ExitResult, FULL_VERSION};
use std::fs::OpenOptions;
use std::{env, fs};
use std::os::unix::fs::{chown, lchown, symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::prelude::*;

// pub const INIT_SCRIPT: &str = include_str!("box-init.sh");

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

fn initialization() -> ExitResult {
    println!("box {}", FULL_VERSION);

    let user = std::env::var("BOX_USER")
        .expect("BOX_USER is undefined");
    let uid = std::env::var("BOX_USER_UID")
        .expect("BOX_USER_UID is undefined");
    let gid = std::env::var("BOX_USER_GID")
        .expect("BOX_USER_GID is undefined");

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
        let dest = Path::new(&home).join(&file);

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

    if Path::new("/bin/sudo").exists() {
        println!("Enabling passwordless sudo for everyone");

        let mut file = OpenOptions::new()
            .write(true)
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

    // TODO run scripts somehow?

    // # run user scripts
    // echo "Running /init.d/ scripts"
    // if [[ -d /init.d ]]; then
    //     for script in /init.d/*; do
    //         if [[ -x "$script" ]]; then
    //             # run each script as the user
    //             if [[ "$HAS_SUDO" -eq 1 ]]; then
    //                 sudo -u "$BOX_USER" "$script"
    //             else
    //                 su - "$BOX_USER" -c "$script"
    //             fi
    //         fi
    //     done
    // fi

    // signalize that init is done
    fs::write("/initialized", "y")
        .unwrap();

    println!("Initialization finished");

    Ok(())
}

pub fn container_init() -> ExitResult {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("Caught termination signal..");

        // stop the loop
        r.store(false, Ordering::SeqCst);
    }).expect("Error while setting signal handler");

    initialization()?;

    while running.load(Ordering::SeqCst) {
        // from my testing the delay from this does not really matter but an empty while generated
        // a high cpu usage which is not ideal in the slightest
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // TODO kill children

    println!("Terminating..");

    Ok(())
}
