use code_docs::DocumentedStruct;
use crate::config::Config;

pub fn show_config_options() {
    let docstring = Config::commented_fields().unwrap();

    println!("Configuration options for each config:\n");
    println!("{}", docstring);
}

