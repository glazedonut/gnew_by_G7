use self::Error::*;
use crate::repo::object::{Change,Commit};
use crate::repo::repository::{FileStatus, Reference, Repository, Status};
use similar::TextDiff;
use std::error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BranchExists,
    CheckoutFailed,
    DirtyWorktree,
    FileNotFound,
    IoError(io::Error),
    MergeFailed(Vec<PathBuf>),
    NoRepository,
    NothingToMerge,
    ObjectCorrupted,
    ObjectMissing,
    ObjectNotFound,
    PushFailed,
    ReferenceNotFound,
    RevisionNotFound,
    RepositoryExists,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BranchExists => write!(f, "branch already exists"),
            CheckoutFailed => write!(f, "commit or remove changes first"),
            DirtyWorktree => write!(f, "dirty work tree"),
            FileNotFound => write!(f, "file not found"),
            IoError(error) => write!(f, "IO error: {}", error),
            MergeFailed(_) => write!(f, "merge failed"),
            NoRepository => write!(f, "no repository at file path"),
            NothingToMerge => write!(f, "nothing to merge"),
            ObjectCorrupted => write!(f, "corrupted object"),
            ObjectMissing => write!(f, "missing object"),
            ObjectNotFound => write!(f, "object not found"),
            PushFailed => write!(f, "local and remote repositories differ, pull first"),
            ReferenceNotFound => write!(f, "reference not found"),
            RevisionNotFound => write!(f, "revision not found"),
            RepositoryExists => write!(
                f,
                "local repository by the same name already exists, delete it first"
            ),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        IoError(err)
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Error {
        IoError(err.into())
    }
}
pub fn print_commit(l:Commit, r:&Repository){
    println!("\x1b[96mcommit {}\x1b[0m", l.hash());
    // if !r.head_hash().is_err(){
    //     println!("HEAD: {} ", r.head_hash().unwrap());
    // }else{
    //     println!("HEAD: detached");
    // }

    for i in (*(*r).branches()).keys(){
        if (*(*r).branches())[i]==l.hash() {
            println!("branch: {} ", i);
        }
    }
    println!("Author: {}", l.author());
    println!("Time: {}", l.time().to_rfc2822());
    println!("Summary:\n{}", l.msg());
}

pub fn print_status(status: &Status) {
    for (path, fstatus) in status {
        match fstatus {
            FileStatus::Unmodified => (),
            _ => println!("{} {}", fstatus.code(), path.display()),
        }
    }
}

pub fn print_heads(r: &Repository) {
    let mut branches: Vec<_> = r.branches().keys().collect();
    branches.sort();

    for branch in branches {
        let current = match r.head() {
            Reference::Branch(b) if b == branch => "*",
            _ => " ",
        };
        println!("{} {}", current, branch)
    }
}

/// Outputs the changes as a unified diff.
pub fn print_diff(changes: &[Change]) -> Result<()> {
    changes.iter().try_for_each(print_file_diff)
}

fn print_file_diff(change: &Change) -> Result<()> {
    let (old, new) = change.contents()?;
    let (a, b) = diff_header(change);

    Ok(TextDiff::from_lines(&old, &new)
        .unified_diff()
        .header(&a.to_string_lossy(), &b.to_string_lossy())
        .to_writer(io::stdout())?)
}

fn diff_header(change: &Change) -> (PathBuf, PathBuf) {
    let (a, b) = match change {
        Change::Add(_) => (None, Some("b")),
        Change::Remove(_) => (Some("a"), None),
        Change::Modify(..) => (Some("a"), Some("b")),
    };
    let header_path = |f| match f {
        None => PathBuf::from("/dev/null"),
        Some(p) => Path::new(p).join(change.path()),
    };
    (header_path(a), header_path(b))
}
