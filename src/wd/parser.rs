use crate::repo::command;
use crate::repo::object::{Hash, Reference, Repository};
use crate::storage::transport::{self, Result};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
enum Gnew {
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
    Status { tree: Hash },
    /// List the heads
    Heads,
    /// Show changes between commits
    Diff {
        #[structopt(max_values = 2)]
        commits: Vec<Hash>,
    },
    /// Output a file at a commit
    Cat { commit: Hash, path: PathBuf },
    /// Update the working directory
    Checkout(CheckoutOptions),
    /// Commit changes to the repository
    Commit { message: String },
    /// Show the commit log
    Log {
        #[structopt(default_value = "0")]
        amount: u32,
    },
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

#[derive(Debug, StructOpt)]
pub struct CheckoutOptions {
    /// The branch or commit to check out
    branch: String,

    /// Create and checkout a new branch
    #[structopt(short = "b")]
    create: bool,

    #[structopt(short, long)]
    force: bool,
}

pub fn init() -> Result<()> {
    command::init()?;
    println!("Initialized empty Gnew repository in .gnew");
    Ok(())
}

pub fn add<P: AsRef<Path>>(paths: &Vec<P>) -> Result<()> {
    command::add(paths)
}

pub fn status(tree: Hash) -> Result<()> {
    let r = Repository::from_disc()?;
    let tree = transport::read_tree(tree)?;

    for (path, status) in r.status(&tree)? {
        println!("{}:\t{:?}", path.display(), status);
    }
    Ok(())
}

pub fn heads() -> Result<()> {
    command::heads()?;
    Ok(())
}

pub fn checkout(o: CheckoutOptions) -> Result<()> {
    let mut r = Repository::from_disc()?;

    if o.create {
        r.create_branch(&o.branch)?;
        println!("Switched to new branch '{}'", o.branch);
    } else if o.branch != "HEAD" {
        let new_head = parse_reference(&o.branch);
        r.checkout(new_head.clone(), o.force)?;
        println!("Switched to {}", new_head);
    }
    Ok(())
}

fn parse_reference(s: &str) -> Reference {
    s.parse()
        .map_or_else(|_| Reference::Branch(s.to_owned()), Reference::Hash)
}

pub fn commit(message: String) -> Result<()> {
    Repository::commit(Some(message))?;
    Ok(())
}

pub fn log(amount: u32) -> Result<()> {
    let log = Repository::log(amount)?;
    for l in log {
        println!("commit {}\n{}", l.hash(), l);
    }
    Ok(())
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
        "blob" => io::stdout().write_all(transport::read_blob(object)?.content())?,
        "tree" => print!("{}", transport::read_tree(object)?),
        "commit" => print!("{}", transport::read_commit(object)?),
        _ => panic!("invalid object type"),
    };
    Ok(())
}

pub fn main() {
    let opt = Gnew::from_args();
    match opt {
        Gnew::Init => init(),
        Gnew::Add { paths } => add(&paths),
        Gnew::Status { tree } => status(tree),
        Gnew::Heads => heads(),
        Gnew::Checkout(opt) => checkout(opt),
        Gnew::Commit { message } => commit(message),
        Gnew::Log { amount } => log(amount),
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
