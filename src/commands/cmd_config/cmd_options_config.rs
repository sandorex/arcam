use code_docs::DocumentedStruct;
use crate::config::Config;
use crate::util;
use crate::vars;

pub fn show_config_options() {
    let docstring = Config::commented_fields()
        .unwrap()
        // replacing vec with array for people that dont know rust
        .replace("Vec<", "Array<");

    // TODO generate the example config with the serialize function instead of
    // raw text so it is always up to date
    println!(r#"ENV {appdir_env}: {appdir:?}

APP DIRECTORY: {appdir:?}
CONFIG DIRECTORY: {cfgdir:?}

--- EXAMPLE CONFIG FILE ---
[[config]]
name = "alpine-example"
image = "docker.io/library/alpine"
network = true
engine_args_podman = [ "--privileged" ]

[[config]]
name = "debian-example"
image = "docker.io/library/debian"
network = true
ports = [
    [8080, 8080]
]
--- EXAMPLE CONFIG FILE ---

--- CONFIG OPTIONS ---
{docstring}
--- CONFIG OPTIONS ---

"#,
        appdir_env=vars::APP_DIR,
        appdir=util::app_dir(),
        cfgdir=util::config_dir(),
    );
}

