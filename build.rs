use vergen_git2::{Emitter, Git2Builder};

fn main() -> anyhow::Result<()> {
    let git2 = Git2Builder::default()
        .describe(false, true, None)
        .sha(false)
        .build()?;

    Emitter::default()
        .add_instructions(&git2)?
        .emit()
}
