use clap::Args;

use super::Command;

/// Templatr rust rewrite (implementation not based on the old one)
#[derive(Args, Clone, Debug)]
pub struct TemplatrCommand {
    /// Link to the git repo, sinonymous with the git clone link
    #[clap(short, long)]
    git: String,

    /// Destination where template will be copied to. FILES WILL BE OVERWRITTEN
    dest: String,

    #[clap(short = 'b', long)]
    git_branch: Option<String>,
}

impl Command for TemplatrCommand {
    fn execute(self) -> color_eyre::Result<()> {
        templatr::prompt::prompt(&self.git, &self.dest, self.git_branch.as_deref())
    }
}
