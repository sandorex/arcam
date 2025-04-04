//! Contains all code that should run inside the container as the init

use crate::command_extensions::*;
use crate::prelude::*;
use crate::FULL_VERSION;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::os::unix::fs::{chown, lchown, symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::{env, fs};

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
                }
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

fn initialization() -> Result<()> {
    println!("{} {}", env!("CARGO_BIN_NAME"), FULL_VERSION);

    let user = std::env::var("HOST_USER").context("HOST_USER is undefined")?;
    let uid = std::env::var("HOST_USER_UID").context("HOST_USER_UID is undefined")?;
    let gid = std::env::var("HOST_USER_GID").context("HOST_USER_GID is undefined")?;

    let uid_u: u32 = uid.parse().unwrap();
    let gid_u: u32 = gid.parse().unwrap();

    let home = format!("/home/{}", user);

    // by default use bash or sh and let user set the shell using other means
    let shell = {
        if Path::new("/bin/bash").exists() {
            "/bin/bash"
        } else {
            // use sh as fallback
            "/bin/sh"
        }
    };

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
                "--shell",
                shell,
                "--home-dir",
                &home,
                "--uid",
                &uid,
                "--user-group",
                "--no-create-home",
                &user,
            ])
            .log_status_anyhow(log::Level::Debug)?
    } else {
        println!("Modifying user {:?}", user);

        Command::new("usermod")
            .args(["--home", &home, "--shell", shell, &user])
            .log_status_anyhow(log::Level::Debug)?
    };

    if !cmd.success() {
        return Err(anyhow!("Error while setting up the user"));
    }

    println!("Setting up the user home");

    // create the home directory if missing
    if !Path::new(&home).exists() {
        fs::create_dir(&home).context("Failed to create user home")?;
    }

    // make sure its own by the user
    chown(&home, Some(uid_u), Some(gid_u)).context("Failed to chown user home directory")?;

    // generate font cache just in case
    {
        println!("Recreating font cache");

        let cmd = Command::new("fc-cache").log_status(log::Level::Debug);

        match cmd {
            Ok(x) => {
                if !x.success() {
                    return Err(anyhow!("Failed to regenerate font cache"));
                }
            }
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
            symlink(source.read_link()?, &dest)?;
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
        let mut perm = fs::symlink_metadata(&dest)?.permissions();

        perm.set_mode(0o700);

        fs::set_permissions(&dest, perm)?
    }

    let has_sudo = Path::new("/bin/sudo").exists();
    if has_sudo {
        println!("Enabling passwordless sudo for everyone");

        let mut file = OpenOptions::new().append(true).open("/etc/sudoers")?;

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

    let init_dir = Path::new(crate::INIT_D_DIR);
    if init_dir.exists() {
        let mut files: Vec<PathBuf> = vec![];
        for entry in init_dir.read_dir().unwrap().flatten() {
            // make sure its executable
            if !entry
                .metadata()
                .is_ok_and(|x| x.permissions().mode() & 0o111 != 0)
            {
                make_executable(&entry.path())?;
            }

            // accept both files and symlinks
            if !entry
                .file_type()
                .is_ok_and(|x| x.is_file() || x.is_symlink())
            {
                continue;
            }

            files.push(entry.path());
        }

        // sort all files to guarantee the correct execution order
        files.sort_by_key(|x| x.file_name().unwrap().to_owned());

        for file in files {
            println!("Executing script {:?}", file);

            // use sudo if available
            if has_sudo {
                Command::new("sudo")
                    .args(["-u", &user])
                    .arg(&file)
                    .log_status_anyhow(log::Level::Debug)
            } else {
                Command::new("su")
                    .args([&user, "-c"])
                    .arg(&file)
                    .log_status_anyhow(log::Level::Debug)
            }
            .with_context(|| anyhow!("Script {:?} has failed", file))?;
        }
    }

    // signalize that init is done
    fs::write(crate::FLAG_FILE_INIT, "y")?;

    println!("Initialization finished");

    Ok(())
}

pub fn container_init() -> Result<()> {
    // create needed directories
    for dir in [crate::ARCAM_DIR, crate::INIT_D_DIR, "/tmp/.X11-unix"] {
        if !Path::new(dir).exists() {
            std::fs::create_dir(dir)?;
        }
    }

    // small wrapper to run as root regardless if sudo is available
    std::fs::write(
        "/bin/asroot",
        r#"#!/bin/sh
set -e

if command -v sudo >/dev/null; then
    sudo -u root -g root -- "$@"
else
    su -c "$*" -g root root
fi
"#,
    )?;
    make_executable(Path::new("/bin/asroot"))?;

    // create the flag file to start preinit
    std::fs::write(crate::FLAG_FILE_PRE_INIT, "y")?;

    // wait for the flag file to be deleted to proceed
    while std::fs::exists(crate::FLAG_FILE_PRE_INIT)? {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    initialization()?;

    // just sleep forever, podman-init will kill it
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }

    #[allow(unreachable_code)]
    Ok(())
}
