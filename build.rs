use vergen_git2::{CargoBuilder, Emitter, Git2Builder, RustcBuilder};

fn main() -> anyhow::Result<()> {
    // cause i cannot figure out how to uppercase a str literal at compile time
    println!(
        "cargo::rustc-env=CARGO_PKG_NAME_UPPERCASE={}",
        env!("CARGO_PKG_NAME").to_ascii_uppercase()
    );

    let git2 = Git2Builder::default().sha(true).build()?;

    let cargo = CargoBuilder::default()
        .debug(true)
        .target_triple(true)
        .features(true)
        .build()?;

    let rustc = RustcBuilder::default().semver(true).build()?;

    Emitter::default()
        .add_instructions(&git2)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .fail_on_error()
        .emit()
}
