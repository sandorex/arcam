use std::path::Path;

use crate::ExitResult;
use crate::util::{app_runtime_dir, command_extensions::*};

fn is_gui_running() -> bool {
    let code = Command::new("systemctl")
        .args(["--user", "status", "box-gui"])
        .output()
        .expect("Could not execute systemctl")
        .to_exitcode();

    match code {
        // the exitcode is 0 / success if the unit exists
        Ok(()) => true,
        // as systemd-run units dissapear after exit this means its not running
        Err(4) => false,
        // there may be other errors, idk
        Err(x) => panic!("Error while checking if gui is runing ({})", x),
    }
}

fn find_free_wayland_index() -> u32 {
    let (uid, _) = crate::util::get_user_uid_gid();

    let mut wl_index: u32 = {
        let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());
        if let Some(x) = display.strip_prefix("wayland-") {
            x.parse().unwrap()
        } else {
            panic!("Failed to parse WAYLAND_DISPLAY env variable: {:?}", display);
        }
    };

    loop {
        // found valid xorg socket
        let path = format!("/run/user/{}/wayland-{}", uid, wl_index);
        if !Path::new(&path).exists() {
            break;
        }

        wl_index += 1;
    }

    wl_index
}

fn find_free_xorg_index() -> u32 {
    let mut xorg_index: u32 = {
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
        if let Some(x) = display.strip_prefix(":") {
            x.parse().unwrap()
        } else {
            panic!("Failed to parse DISPLAY env variable: {:?}", display);
        }
    };

    loop {
        // found valid xorg socket
        let path = format!("/tmp/.X11-unix/X{}", xorg_index);
        if !Path::new(&path).exists() {
            break;
        }

        xorg_index += 1;
    }

    xorg_index
}

pub fn get_gui_sockets() -> Option<(u32, u32)> {
    match std::fs::read_to_string(app_runtime_dir().join("box-gui")) {
        Ok(x) => {
            // FIXME TODO this is a terrible way to do it
            let mut lines = x.lines();
            let xorg_socket: u32 = lines.nth(0).unwrap().parse().unwrap();
            let wl_socket: u32 = lines.nth(0).unwrap().parse().unwrap();

            Some((xorg_socket, wl_socket))
        },
        _ => None,
    }
}

/// Start gui if it is not already running
pub fn start_gui(dry_run: bool) -> ExitResult {
    // do not start it twice
    if is_gui_running() {
        return Ok(());
    }

    // TODO current implementation HOPES that weston select next wayland and xorg socket index,
    // this is basically waiting to fail, find another way to do it!
    //
    // There is no way to set DISPLAY but there is way to set WAYLAND_DISPLAY...

    let xorg_socket_index = find_free_xorg_index();
    let wayland_socket_index = find_free_wayland_index();

    let runtime_dir = app_runtime_dir();
    let gui_file = runtime_dir.join("box-gui");

    let mut cmd = Command::new("systemd-run");
    cmd.args([
        "--user",
        "--unit=box-gui",
        "--",
        "weston",
        "--no-config",
        "--xwayland",
        // TODO add a way to specify other backends like pipewire, vnc, rdp
        "--backend=wayland",
        "--shell=desktop",
        // do not print logs
        "--flight-rec-scopes=",
    ]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        // create the runtime directory
        let _ = std::fs::create_dir_all(runtime_dir);

        // write socket indexes
        std::fs::write(gui_file, format!("{}\n{}\n", xorg_socket_index, wayland_socket_index))
            .expect("Error writing to gui_file");

        cmd
            .output()
            .expect("Could not execute systemd-run")
            .to_exitcode()
    }
}
