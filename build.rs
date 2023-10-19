use color_eyre::Result;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    color_eyre::install()?;
    // Generate the default 'cargo:' instruction output
    // Emit the instructions
    EmitBuilder::builder()
        .all_git()
        .emit()
        .expect("vergen failed");
    Ok(())
}
