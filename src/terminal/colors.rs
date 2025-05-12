use owo_colors::{FgColorDisplay, OwoColorize};

/// Defines a color scheme for Spiggy
pub trait QPMColor: OwoColorize {
    #[inline(always)]
    fn download_file_name_color(&self) -> FgColorDisplay<owo_colors::colors::Cyan, Self> {
        self.cyan()
    }

    #[inline(always)]
    fn file_path_color(&self) -> FgColorDisplay<owo_colors::colors::Yellow, Self> {
        self.yellow()
    }

    #[inline(always)]
    fn dependency_id_color(&self) -> FgColorDisplay<owo_colors::colors::Blue, Self> {
        self.blue()
    }

    #[inline(always)]
    fn version_id_color(&self) -> FgColorDisplay<owo_colors::colors::Blue, Self> {
        self.blue()
    }

    #[inline(always)]
    fn dependency_version_color(&self) -> FgColorDisplay<owo_colors::colors::Magenta, Self> {
        self.purple()
    }

    #[inline(always)]
    fn alternate_dependency_version_color(
        &self,
    ) -> FgColorDisplay<owo_colors::colors::Yellow, Self> {
        self.yellow()
    }
}

impl<D: OwoColorize> QPMColor for D {}
