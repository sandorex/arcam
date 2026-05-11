use std::{collections::HashMap, sync::LazyLock};

use crate::command_ext::command_extensions::*;

/// Simple yes/no prompt
pub fn prompt(prompt: &str) -> bool {
    use std::io::Write;
    let mut s = String::new();

    // if not yes then yes, but if yes then no yes
    print!("{} [y/N] ", prompt);

    let _ = std::io::stdout().flush();

    std::io::stdin()
        .read_line(&mut s)
        .expect("Could not read stdin");
    s = s.trim().to_string();

    matches!(s.to_lowercase().as_str(), "y" | "yes")
}

/// Check whether executable exists in PATH
pub fn executable_in_path(cmd: &str) -> bool {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("which {}", cmd))
        .log_output()
        .expect("Failed to execute 'which'");

    output.status.success()
}

/// Check if running inside a container
pub fn is_in_container() -> bool {
    use std::env;
    use std::path::Path;

    Path::new("/run/.containerenv").exists()
        || Path::new("/.dockerenv").exists()
        || env::var("container").is_ok()
}

/// Expands vars from map using bash-like syntax `$HOME` and `${HOME}`, also supports default and
/// error suffixes like in bash `${HOME:?}` (error if undefined) and `${HOME:-default}` where
/// "default" will be the value if undefined
pub fn expand_vars<F>(input: &str, mut getter: F) -> anyhow::Result<String>
    where
        F: FnMut(&str) -> Option<String>
{
    // compile regex only once
    static RE: LazyLock<regex::Regex> = LazyLock::new(||
        regex::Regex::new(r"\$\{([^}]*)\}|\$([A-Za-z_][A-Za-z0-9_]*\b|\$)"
    ).unwrap());

    let mut output = String::with_capacity(input.len() + 256);
    let mut last_range_end = 0usize;

    for m in RE.captures_iter(input) {
        let full = m.get(0).unwrap();
        let curly = m.get(1).is_some();

        let name = m.get(1).unwrap_or_else(|| m.get(2).unwrap()).as_str();
        let (name, default): (&str, Option<&str>) = if curly {
            if name.ends_with(":?") {
                // no default value error
                (&name[..name.len() - 2], None)
            } else if name.contains(":-") {
                let (name, default) = name.split_once(":-").unwrap();

                // use specified default value
                (name, Some(default))
            } else {
                // by default subtitute with empty string
                (name, Some(""))
            }
        } else if name == "$" {
            // escape the sign by just doing '$$'
            ("", Some("$"))
        } else {
            (name, Some(""))
        };

        // if name is empty then just use default
        let value = if name.is_empty() {
            default.map(|x| x.to_owned())
        } else {
            getter(name)
                .or_else(|| default.map(|x| x.to_owned()))
        };

        // if still None then just error out
        let Some(value) = value else {
            return Err(anyhow::anyhow!("Error env variable {name:?} is undefined"));
        };

        // push left part of the input
        output.push_str(&input[last_range_end..full.start()]);

        // push the replacement
        output.push_str(&value);

        // save the end position
        last_range_end = full.end();
    }

    // push the rest of the string
    output.push_str(&input[last_range_end..]);

    Ok(output)
}

/// Wrapper around `expand_vars` to use a map instead of a function
#[allow(unused)]
pub fn expand_vars_map(input: &str, values: &HashMap<String, String>) -> anyhow::Result<String> {
    expand_vars(input, |x| {
        values.get(x).cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_expansion() {
        assert_eq!(
            expand_vars_map(
                "One !#F# $# @{  } $one ${two} ${three:-} $four ${four:?} five",
                &HashMap::from([
                    ("one".to_owned(), "1".to_owned()),
                    ("two".to_owned(), "2".to_owned()),
                    ("three".to_owned(), "3".to_owned()),
                    ("four".to_owned(), "4".to_owned()),
                ])
            ).unwrap(),
            "One !#F# $# @{  } 1 2 3 4 4 five".to_owned()
        );

        // make sure by default it expands to nothing
        assert_eq!(
            expand_vars_map(
                "$one ${one} ${one:-2}",
                &HashMap::from([])
            ).unwrap(),
            "  2".to_owned()
        );

        // make sure it errors out if empty
        assert!(expand_vars_map(
            "${one:?}",
            &HashMap::from([])
        ).is_err());

        assert_eq!(
            expand_vars_map(
                "/${one:?}/",
                &HashMap::from([
                    ("one".to_owned(), "1".to_owned())
                ])
            ).unwrap(),
            "/1/".to_owned()
        );

        assert_eq!(
            expand_vars_map(
                "cd $HOME/.config",
                &HashMap::from([
                    ("HOME".to_owned(), "/home/user".to_owned())
                ])
            ).unwrap(),
            "cd /home/user/.config".to_owned()
        );

        assert_eq!(
            expand_vars_map(
                "cd $HOME/.config/${CFG_DIR:-arcam}/config.toml",
                &HashMap::from([
                    ("HOME".to_owned(), "/home/user".to_owned())
                ])
            ).unwrap(),
            "cd /home/user/.config/arcam/config.toml".to_owned()
        );

        assert_eq!(
            expand_vars_map(
                "/$${x}/$$xx",
                &HashMap::from([])
            ).unwrap(),
            "/${x}/$xx".to_owned()
        );
    }
}
