use code_docs::DocumentedStruct;
use crate::config::Config;
use crate::prelude::*;

pub fn show_config_options(ctx: Context) {
    let docstring = Config::commented_fields()
        .unwrap()
        // replacing vec with array for people that dont know rust
        .replace("Vec<", "Array<");

    // TODO generate the example config with the serialize function instead of
    // raw text so it is always up to date
    println!(r#"APP DIRECTORY (ENV {appdir_env}): {appdir:?}
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
        appdir_env=crate::ENV_APP_DIR,
        appdir=ctx.app_dir,
        cfgdir=ctx.config_dir(),
    );
}
