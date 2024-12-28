use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    // Generate the default 'cargo:' instruction output
    // Emit the instructions
    #[cfg(feature = "cli")]
    {
        use vergen::EmitBuilder;

        EmitBuilder::builder()
            .all_git()
            .emit()
            .expect("vergen failed");
    }
    Ok(())
}
