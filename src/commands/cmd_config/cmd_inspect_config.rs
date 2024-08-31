use crate::config::ConfigFile;
use crate::cli::cli_config::CmdConfigInspectArgs;
use crate::ExitResult;

pub fn inspect_config(cli_args: &CmdConfigInspectArgs) -> ExitResult {
    match ConfigFile::load_from_file(&cli_args.path) {
        Ok(x) => {
            println!("{:#?}", x);

            Ok(())
        },
        Err(err) => {
            // NOTE err is custom error so the message is already predefined
            eprintln!("{}", err);

            Err(1)
        }
    }
}
