/// Contains all code that should run inside the container as the init

use std::process::ExitCode;
use crate::FULL_VERSION;

pub const INIT_SCRIPT: &str = include_str!("box-init.sh");

pub fn container_init() -> ExitCode {
    use std::process::Command;
    use std::os::unix::process::CommandExt;
    use std::os::unix::fs::PermissionsExt;
    use std::fs;
    use std::path::Path;
    use std::io::Write;

    println!("box {}", FULL_VERSION);

    // open init file for rw
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(Path::new("/init"))
        .expect("Error while creating /init");

    // set the correct permissions
    let mut perms = file.metadata()
        .expect("Cannot get metadata of /init")
        .permissions();

    // make it executable
    perms.set_mode(0o755);

    file.set_permissions(perms)
        .expect("Error while setting permissions for /init");

    // write the init script to it
    file.write_all(INIT_SCRIPT.as_bytes())
        .expect("Error while writing to /init");

    // forcibly drop it so it gets written to disk
    drop(file);

    // execute it and replace this process with it
    let cmd = Command::new("/init")
        .exec();

    // NOTE everything here will only be executed if exec fails!

    eprintln!("Error while executing /init: {:?}", cmd);

    ExitCode::FAILURE
}
