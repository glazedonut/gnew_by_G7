use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
pub enum Gnew {
    /// Create an empty repository
    Init,
    /// Copy an existing repository
    Clone,
    /// Add files to tracking list
    Add {
        #[structopt(parse(from_os_str))]
        files: Vec<PathBuf>,
    },
    /// Remove files from tracking list
    Remove {},
    /// Show the repository status
    Status,
    /// List the heads
    Heads,
    /// Show changes between commits
    Diff {},
    /// Output a file at a commit
    Cat {},
    /// Check out a commit
    Checkout {},
    /// Commit changes to the repository
    Commit {
        #[structopt(short)]
        message: Option<String>,
    },
    /// Show the commit log
    Log,
    /// Merge two commits
    Merge {},
    /// Pull changes from another repository
    Pull {},
    /// Push changes to another repository
    Push {},
}

pub fn parse() -> Gnew {
    Gnew::from_args()
}
