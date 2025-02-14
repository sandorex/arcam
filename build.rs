use vergen_git2::{Emitter, Git2Builder};

fn main() -> anyhow::Result<()> {
    let git2 = Git2Builder::default()
        .branch(true)
        .describe(false, true, None)
        .sha(false)
        .build()?;

    // cause i cannot figure out how to uppercase a str literal at compile time
    println!(
        "cargo::rustc-env=CARGO_PKG_NAME_UPPERCASE={}",
        env!("CARGO_PKG_NAME").to_ascii_uppercase()
    );

    Emitter::default().add_instructions(&git2)?.emit()
}
