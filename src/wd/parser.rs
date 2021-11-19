use crate::repo::command;
use crate::repo::object::{Hash, Repository};
use crate::storage::transport::{self, Result};
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
    /// Write a tree object from the working directory
    WriteTree,
    /// Show the content of an object
    CatObject {
        /// Object type
        #[structopt(possible_values = &["blob", "tree", "commit"])]
        type_: String,

        /// Object hash
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

pub fn add<P: AsRef<Path>>(paths: &Vec<P>) -> Result<()> {
    command::add(paths)
}

pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<()> {
    println!("{}", transport::write_blob(path)?.hash());
    Ok(())
}

pub fn write_tree() -> Result<()> {
    println!("{}", Repository::from_disc()?.write_tree()?.hash());
    Ok(())
}

pub fn cat_object(type_: &str, object: Hash) -> Result<()> {
    match type_ {
        "blob" => {
            let blob = transport::read_blob(object)?;
            io::stdout().write_all(blob.content())?;
        }
        "tree" => print!("{}", transport::read_tree(object)?),
        "commit" => todo!(),
        _ => panic!("invalid object type"),
    };
    Ok(())
}

pub fn main() {
    let opt = parse();
    match opt {
        Gnew::Init => init(),
        Gnew::Add { paths } => add(&paths),
        Gnew::HashFile { path } => hash_file(path),
        Gnew::WriteTree => write_tree(),
        Gnew::CatObject { type_, object } => cat_object(&type_, object),
        _ => todo!(),
    }
    .unwrap_or_else(|err| {
        eprintln!("fatal: {}", err);
        std::process::exit(1)
    })
}
