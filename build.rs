use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    // Generate the default 'cargo:' instruction output
    // Emit the instructions
    #[cfg(feature = "cli")]
    {
        use vergen_gitcl::{Emitter, Gitcl};

        Emitter::default()
            .add_instructions(&Gitcl::all_git())
            .and_then(|emitter| emitter.emit())
            .expect("vergen failed");
    }
    Ok(())
}
