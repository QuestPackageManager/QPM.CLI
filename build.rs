use color_eyre::Result;
use vergen::{vergen, Config};

fn main() -> Result<()> {
    color_eyre::install()?;
    // Generate the default 'cargo:' instruction output
    vergen(Config::default()).expect("Vergen failed");
    Ok(())
}
