use crate::repo::object::{Hash, Tree};
use crate::repo::repository::{MergeStrategy, Reference, Repository};
use crate::storage::transport;
use crate::wd::ui::{self, Error, Result};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
enum Gnew {
    /// Create an empty repository
    Init,
    /// Copy an existing repository
    Clone { repository: PathBuf },
    /// Add files to tracking list
    Add {
        #[structopt(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Remove files from tracking list
    Remove {
        #[structopt(required = true)]
        paths: Vec<PathBuf>,
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
    Merge { commit: String },
    /// Pull changes from another repository
    Pull {
        repository: PathBuf,

        #[structopt(short, long)]
        all: bool,
    },
    /// Push changes to another repository
    Push {
        repository: PathBuf,

        #[structopt(short, long)]
        all: bool,
    },

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
    let r = Repository::init()?;
    println!(
        "Initialized empty Gnew repository in {}",
        r.storage_dir().display()
    );
    Ok(())
}

pub fn clone<P: AsRef<Path> + Copy>(rep: P) -> Result<()> {
    Repository::clone(rep)?;
    Ok(())
}

pub fn add<P: AsRef<Path>>(paths: &Vec<P>) -> Result<()> {
    let mut r = Repository::open()?;
    r.add(paths)?;

    Ok(())
}

pub fn remove<P: AsRef<Path>>(paths: &Vec<P>) -> Result<()> {
    let mut r = Repository::open()?;
    r.remove(paths)?;

    Ok(())
}

pub fn status() -> Result<()> {
    let r = Repository::open()?;

    let tree = match r.head_hash() {
        Ok(c) => transport::read_commit(c)?.tree()?,
        Err(_) => Tree::new(),
    };
    ui::print_status(&r.status(&tree)?);

    Ok(())
}

pub fn heads() -> Result<()> {
    let r = Repository::open()?;
    ui::print_heads(&r);
    Ok(())
}

pub fn diff(commits: &[String]) -> Result<()> {
    let r = Repository::open()?;

    let changes = match commits {
        [] => {
            let tree = match r.head_hash() {
                Ok(c) => transport::read_commit(c)?.tree()?,
                Err(_) => return Ok(()),
            };
            r.diff_worktree(&tree)
        }
        [c1] => {
            let c1 = r.rev_parse(c1)?;
            let tree = transport::read_commit(c1)?.tree()?;
            r.diff_worktree(&tree)
        }
        [c1, c2] => {
            let c1 = r.rev_parse(c1)?;
            let c2 = r.rev_parse(c2)?;
            let t1 = transport::read_commit(c1)?.tree()?;
            let t2 = transport::read_commit(c2)?.tree()?;
            t1.diff(&t2)
        }
        _ => panic!("too many arguments"),
    }?;
    ui::print_diff(&changes)?;

    Ok(())
}

pub fn cat(commit: String, path: &Path) -> Result<()> {
    let r = Repository::open()?;
    let c = r.rev_parse(&commit)?;
    let file = transport::read_commit(c)?.tree()?.file(path)?;
    io::stdout().write_all(&file.contents()?)?;
    Ok(())
}

pub fn checkout(o: CheckoutOptions) -> Result<()> {
    let mut r = Repository::open()?;
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
    let mut r = Repository::open()?;
    println!("{}", r.commit(message)?.hash());
    Ok(())
}

pub fn log(amount: u32) -> Result<()> {
    let r = Repository::open()?;
    let log = r.log(amount)?;
    for l in log {
        ui::print_commit(l,&r);
    }
    Ok(())
}

pub fn merge(commit: String) -> Result<()> {
    let mut r = Repository::open()?;

    match r.merge(r.rev_parse(&commit)?) {
        Ok(MergeStrategy::FastForward) => println!("Fast-forward"),
        Ok(_) => println!("Merge complete: remember to commit."),
        Err(Error::MergeFailed(conflicts)) => {
            for path in conflicts {
                eprintln!("Merge conflict in {}", path.display())
            }
            eprintln!("Merge failed: fix conflicts and commit the result.");
            std::process::exit(1)
        }
        Err(err) => return Err(err),
    };
    Ok(())
}

pub fn pull<P: AsRef<Path>>(path: P, all: bool) -> Result<()> {
    let mut r = Repository::open()?;
    r.pull(path, all)?;
    Ok(())
}

pub fn push<P: AsRef<Path>>(path: P, all: bool) -> Result<()> {
    let r = Repository::open()?;
    r.push(path, all)?;
    Ok(())
}

pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<()> {
    println!("{}", transport::write_blob(path)?.hash());
    Ok(())
}

pub fn write_tree() -> Result<()> {
    println!("{}", Repository::open()?.write_tree()?.hash());
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
        Gnew::Clone { repository } => clone(&repository),
        Gnew::Add { paths } => add(&paths),
        Gnew::Remove { paths } => remove(&paths),
        Gnew::Status => status(),
        Gnew::Heads => heads(),
        Gnew::Diff { commits } => diff(&commits),
        Gnew::Cat { commit, path } => cat(commit, &path),
        Gnew::Checkout(opt) => checkout(opt),
        Gnew::Commit { message } => commit(message),
        Gnew::Log { amount } => log(amount),
        Gnew::Merge { commit } => merge(commit),
        Gnew::Pull { repository, all } => pull(repository, all),
        Gnew::Push { repository, all } => push(repository, all),
        Gnew::HashFile { path } => hash_file(path),
        Gnew::WriteTree => write_tree(),
        Gnew::CatObject { type_, object } => cat_object(&type_, object),
    }
    .unwrap_or_else(|err| {
        eprintln!("fatal: {}", err);
        std::process::exit(1)
    })
}
