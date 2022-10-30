use owo_colors::{FgColorDisplay, OwoColorize};


/// Defines a color scheme for Spiggy
pub trait QPMColor: OwoColorize {
    #[inline(always)]
    fn download_file_name_color(&self) -> FgColorDisplay<owo_colors::colors::Cyan, Self> {
        self.cyan()
    }


}

impl<D: OwoColorize> QPMColor for D {}