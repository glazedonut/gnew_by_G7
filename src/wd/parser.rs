use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
pub enum Gnew {
    /// Create an empty repository
    Init,
    /// Copy an existing repository
    Clone {
        repository: PathBuf,
        directory: PathBuf,
    },
    /// Add files to tracking list
    Add {
        #[structopt(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Remove files from tracking list
    Remove {
        #[structopt(required = true)]
        files: Vec<PathBuf>,

        #[structopt(short)]
        recursive: bool,
    },
    /// Show the repository status
    Status,
    /// List the heads
    Heads,
    /// Show changes between commits
    Diff {
        #[structopt(max_values = 2)]
        commits: Vec<String>,
    },
    /// Output a file at a commit
    Cat { commit: String, path: PathBuf },
    /// Check out a commit
    Checkout { commit: String },
    /// Commit changes to the repository
    Commit {
        #[structopt(short)]
        message: Option<String>,
    },
    /// Show the commit log
    Log,
    /// Merge two commits
    Merge { commit: String },
    /// Pull changes from another repository
    Pull { repository: PathBuf },
    /// Push changes to another repository
    Push { repository: PathBuf },
}

pub fn parse() -> Gnew {
    Gnew::from_args()
}
