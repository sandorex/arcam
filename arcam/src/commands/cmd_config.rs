use crate::cli::{CmdConfigArgs, ConfigArg};
use crate::command_extensions::*;
use crate::config::{Config, ConfigFile, Source};
use crate::prelude::*;

fn get_image_config(ctx: &Context, image: &str) -> Result<String> {
    let cmd = ctx
        .engine
        .command()
        .args(["image", "exists", image])
        .log_output()
        .expect(crate::ENGINE_ERR_MSG);

    if !cmd.status.success() {
        return Err(anyhow!("Image {:?} does not exist", image));
    }

    // TODO this is pretty engine specific, probably should abstract it
    let mut cmd = ctx.engine.command();
    cmd.args([
        "run",
        "--rm",
        "-it",
        // basically just cat the file, should be pretty portable
        "--entrypoint",
        "cat",
        image,
        crate::ARCAM_CONFIG,
    ]);

    let output = cmd.log_output().expect(crate::ENGINE_ERR_MSG);

    if !output.status.success() {
        return Err(anyhow!("Failed to extract config from image {:?}", image));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn show_example(ctx: &Context) -> Result<()> {
    // NOTE instead of writing examples by hand im serializing it here
    let example: String = {
        let example = ConfigFile::latest(Config {
            source: Source::Image("docker.io/library/debian:latest".into()),
            network: true,
            engine_args: vec!["--privileged".into()],
            ports: vec![(8080, 8080), (6666, 6666)],
            env: vec![("LS_COLORS".into(), "rs=0:di=01;34:ln=01;...".into())],
            ..Default::default()
        });

        toml::to_string(&example)?
    };

    println!(
        r#"APP DIRECTORY (ENV {appdir_env}): {appdir:?}
CONFIG DIRECTORY: {cfgdir:?}

-- EXAMPLE --
{example}
-- EXAMPLE --"#,
        appdir_env = crate::ENV_APP_DIR,
        appdir = ctx.app_dir,
        cfgdir = ctx.config_dir(),
    );

    Ok(())
}

pub fn config_command(ctx: Context, cli_args: CmdConfigArgs) -> Result<()> {
    // show options and quit
    if cli_args.options {
        println!("{}", Config::docs());
        return Ok(());
    }

    // show example and quit
    if cli_args.example {
        return show_example(&ctx);
    }

    match cli_args.config.unwrap() {
        ConfigArg::File(x) => {
            println!("Inspecting config from file");
            println!("{:#?}", ConfigFile::config_from_file(&x));
        },

        // if image is passed extract from image
        ConfigArg::Image(image) => {
            println!("Inspecting config from image {:?}", image);

            let raw = get_image_config(&ctx, &image)?;
            println!("{:#?}", crate::config::ConfigFile::config_from_str(&raw)?);
        },

        ConfigArg::Config(name) => {
            println!("Inspecting config");
            println!("{:#?}", ctx.find_config(&name)?);
        },

        ConfigArg::Nix(..) => {
            println!("Cannot inspect nix");
        },
    };

    Ok(())
}
