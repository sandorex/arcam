use crate::cli::{CmdConfigArgs, ConfigArg};
use crate::command_extensions::*;
use crate::config::{Config, ConfigFile};
use crate::prelude::*;
use code_docs::DocumentedStruct;

fn get_image_config(ctx: &Context, image: &str) -> Result<String> {
    let cmd = ctx
        .engine
        .command()
        .args(["image", "exists", image])
        .log_output(log::Level::Debug)
        .expect(crate::ENGINE_ERR_MSG);

    if !cmd.status.success() {
        return Err(anyhow!("Image {:?} does not exist", image));
    }

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

    let output = cmd
        .log_output(log::Level::Debug)
        .expect(crate::ENGINE_ERR_MSG);

    if !output.status.success() {
        return Err(anyhow!("Failed to extract config from image {:?}", image));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn show_options() -> Result<()> {
    let docstring = Config::commented_fields()
        .unwrap()
        // replacing vec with array for people that dont know rust
        .replace("Vec<", "Array<");

    println!(r#"{docstring}"#);

    Ok(())
}

fn show_example(ctx: &Context) -> Result<()> {
    // NOTE instead of writing examples by hand im serializing it here
    let example: String = {
        let example = ConfigFile {
            version: 1,
            config: Config {
                image: "docker.io/library/debian:latest".into(),
                network: true,
                engine_args: vec!["--privileged".into()],
                ports: vec![(8080, 8080), (6666, 6666)],
                env: vec![("LS_COLORS".into(), "rs=0:di=01;34:ln=01;...".into())],
                ..Default::default()
            },
        };

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
        return show_options();
    }

    // show example and quit
    if cli_args.example {
        return show_example(&ctx);
    }

    let config = cli_args.config.unwrap();

    // if image is passed extract from image
    if let ConfigArg::Image(image) = &config {
        println!("Inspecting config from image {:?}", image);

        let raw = get_image_config(&ctx, image)?;
        println!("{:#?}", crate::config::ConfigFile::config_from_str(&raw)?);

        return Ok(());
    }

    println!("Inspecting config");
    println!("{:#?}", config.into_config(&ctx)?);

    Ok(())
}
