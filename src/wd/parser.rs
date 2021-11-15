use crate::repo::command;
use crate::repo::object::Hash;
use crate::storage::transport::Result;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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
        commits: Vec<Hash>,
    },
    /// Output a file at a commit
    Cat { commit: Hash, path: PathBuf },
    /// Check out a commit
    Checkout { commit: Hash },
    /// Commit changes to the repository
    Commit {
        #[structopt(short)]
        message: Option<String>,
    },
    /// Show the commit log
    Log,
    /// Merge two commits
    Merge { commit: Hash },
    /// Pull changes from another repository
    Pull { repository: PathBuf },
    /// Push changes to another repository
    Push { repository: PathBuf },

    // Low-level commands
    //
    /// Write a blob object from a file
    HashFile {
        /// The file to hash
        path: PathBuf,
    },

    /// Show the content of an object
    CatObject {
        /// The object to show
        object: Hash,
    },
}

pub fn parse() -> Gnew {
    Gnew::from_args()
}

pub fn init() -> Result<()> {
    command::init()?;
    println!("Initialized empty Gnew repository in .gnew");
    Ok(())
}

pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<()> {
    command::hash_file(path).map(|blob| println!("{}", blob.hash()))
}

pub fn cat_object(object: &Hash) -> Result<()> {
    io::stdout().write_all(&command::cat_object(object)?)?;
    Ok(())
}

pub fn main() {
    let opt = parse();
    match &opt {
        Gnew::Init => init(),
        Gnew::HashFile { path } => hash_file(path),
        Gnew::CatObject { object } => cat_object(object),
        _ => todo!(),
    }
    .unwrap_or_else(|err| {
        eprintln!("fatal: {}", err);
        std::process::exit(1)
    })
}
