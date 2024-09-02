use vergen_git2::{Emitter, BuildBuilder, Git2Builder};

fn main() -> anyhow::Result<()> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let git2 = Git2Builder::default().describe(false, true, None).build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&git2)?
        .emit()
}
