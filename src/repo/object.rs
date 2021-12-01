use crate::storage::serialize::serialize_blob;
use crate::storage::transport::Error::*;
use crate::storage::transport::{self, write_commit, write_empty_repo, Result, read_tree};
use chrono::{DateTime, TimeZone, Utc};
use sha1::{self, Sha1};
use std::collections::HashMap;
use std::{env};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::result;

use std::str;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec;
use walkdir::{self, DirEntry, WalkDir};

const MAX_TREE_DEPTH: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hash(sha1::Digest);

#[derive(Debug)]
pub struct Repository {
    head: Reference,
    branches: HashMap<String, Hash>,
    pub tracklist: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Reference {
    Hash(Hash),
    Branch(String),
}

pub type Status = HashMap<PathBuf, FileStatus>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileStatus {
    Untracked,
    Unmodified,
    Modified,
    Added,
    /// File was removed from tracking list.
    Deleted,
    /// File is tracked but missing from working tree.
    Missing,
}

#[derive(Debug, PartialEq)]
pub struct Commit {
    hash: Hash,
    pub(crate) tree: Hash,
    parent: Option<Hash>,
    author: String,
    time: DateTime<Utc>,
    msg: String,
}

/// Commit metadata used to create a commit object.
#[derive(Debug, PartialEq)]
pub struct CommitInfo {
    pub tree: Hash,
    pub parent: Option<Hash>,
    pub author: String,
    pub time: DateTime<Utc>,
    pub msg: String,
}

#[derive(Debug, PartialEq)]
pub struct Tree {
    hash: Hash,
    entries: Vec<TreeEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeEntry {
    kind: TreeEntryKind,
    hash: Hash,
    name: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TreeEntryKind {
    Blob,
    Tree,
}

/// A file stored in a tree object.
#[derive(Debug, PartialEq)]
pub struct File {
    pub path: PathBuf,
    pub hash: Hash,
}

/// The hashed contents of a file.
#[derive(Debug, PartialEq)]
pub struct Blob {
    hash: Hash,
    content: Vec<u8>,
}

impl Hash {
    pub fn new() -> Hash {
        Hash(Sha1::new().digest())
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0 = Sha1::from(data).digest()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl str::FromStr for Hash {
    type Err = sha1::DigestParseError;

    fn from_str(s: &str) -> result::Result<Hash, sha1::DigestParseError> {
        Ok(Hash(s.parse()?))
    }
}

impl Repository {
    pub fn new() -> Repository {
        Repository {
            head: Reference::Branch("main".to_owned()),
            branches: HashMap::new(),
            tracklist: Vec::<String>::new(),
        }
    }

    // creates empty repository and writes it to disc
    pub fn create_empty() -> Result<Repository> {
        let r = Repository::new();

        write_empty_repo()?;

        Ok(r)
    }

    pub fn from_disc() -> Result<Repository> {
        Ok(Repository {
            head: transport::read_head()?,
            branches: transport::read_branches()?,
            tracklist: transport::read_tracklist()?,
        })
    }

    pub fn head(&self) -> &Reference {
        &self.head
    }

    pub fn head_hash(&self) -> Result<Hash> {
        self.resolve_reference(&self.head)
    }

    fn resolve_reference(&self, r: &Reference) -> Result<Hash> {
        match r {
            Reference::Hash(hash) => Ok(*hash),
            Reference::Branch(b) => self.branch(b),
        }
    }

    fn set_head(&mut self, head: Reference) -> Result<()> {
        transport::write_head(&head)?;
        Ok(self.head = head)
    }

    pub fn branch(&self, name: &str) -> Result<Hash> {
        self.branches.get(name).copied().ok_or(ReferenceNotFound)
    }

    pub fn branches(&self) -> &HashMap<String, Hash> {
        &self.branches
    }

    fn set_branch(&mut self, name: &str, hash: Hash) -> Result<()> {
        transport::write_branch(name, hash)?;
        self.branches.insert(name.to_owned(), hash);
        Ok(())
    }

    /// Updates HEAD to point to a new branch.
    pub fn create_branch(&mut self, name: &str) -> Result<()> {
        if self.branches.contains_key(name) {
            return Err(BranchExists);
        }
        if let Ok(hash) = self.head_hash() {
            self.set_branch(name, hash)?;
        }
        self.set_head(Reference::Branch(name.to_owned()))
    }

    /// Checks if a file is tracked.
    pub fn is_tracked(&self, path: &Path) -> bool {
        self.tracklist.contains(&path.to_str().unwrap().to_owned())
    }

    /// Returns the working tree status.
    pub fn status(&self, tree: &Tree) -> Result<Status> {
        let mut status = HashMap::new();
        let mut head_files = HashMap::new();

        for f in tree.files() {
            let File { path, hash } = f?;
            head_files.insert(path, hash);
        }
        for f in self.walk_worktree() {
            let f = f?;
            let path = f.path().strip_prefix(".").unwrap();

            let fstatus = match (head_files.get(path), self.is_tracked(path)) {
                (None, true) => FileStatus::Added,
                (None, false) => FileStatus::Untracked,
                (Some(hash), true) => {
                    if hash_file(path)? == *hash {
                        FileStatus::Unmodified
                    } else {
                        FileStatus::Modified
                    }
                }
                (Some(_), false) => FileStatus::Deleted,
            };
            status.insert(path.to_owned(), fstatus);
        }
        for path in head_files.keys() {
            if !status.contains_key(path) {
                let fstatus = if self.is_tracked(path) {
                    FileStatus::Missing
                } else {
                    FileStatus::Deleted
                };
                status.insert(path.to_owned(), fstatus);
            }
        }
        Ok(status)
    }

    /// Writes a tree object from the working directory.
    pub fn write_tree(&self) -> Result<Tree> {
        let mut tree = self.write_tree_rec(".".as_ref())?;
        transport::write_tree(&mut tree)?;
        Ok(tree)
    }

    fn write_tree_rec(&self, dir: &Path) -> Result<Tree> {
        let mut tree = Tree::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.starts_with("./.gnew") {
                continue;
            }
            let fname = entry.file_name().to_str().unwrap().to_owned();

            if entry.file_type()?.is_dir() {
                let mut subtree = self.write_tree_rec(&path)?;
                if !subtree.is_empty() {
                    transport::write_tree(&mut subtree)?;
                    tree.add_tree(subtree.hash(), fname)
                }
            } else if self.is_tracked(path.strip_prefix(".").unwrap()) {
                tree.add_blob(transport::write_blob(path)?.hash(), fname)
            }
        }
        Ok(tree)
    }

    pub fn commit(commitmsg: Option<String>) -> Result<()> {
        let mut _cmsg: String = "".to_string();
        _cmsg = match commitmsg {
            Some(ref c) => c.to_string(),
            None => "".to_string(),
        };
        let mut r = Repository::from_disc()?;

        let newtree: Result<Tree> = Repository::write_tree(&r);
        let _treehash = match newtree {
            Ok(c) => c.hash,
            Err(..) => Hash::new(),
        };
        let mut user: String = "Temp user".to_string();
        let env_vars = env::vars();
        for (key, value) in env_vars.into_iter() {
            if key == "USER".to_string() {
                user = value;
            }
        }
        let time = SystemTime::now().duration_since(UNIX_EPOCH);
        let currtime = match time {
            Ok(c) => c.as_millis(),
            Err(_) => 0,
        };
        let _date: DateTime<Utc> = Utc.timestamp(currtime as i64, 0);
        let _newcommit = CommitInfo {
            tree: _treehash,
            parent: r.head_hash().ok(),
            author: user,
            time: _date,
            msg: _cmsg,
        };
        let mut commit = Commit::new(_newcommit);
        let _resultcommit = write_commit(&mut commit);

        r.update_head(commit.hash())
    }

    fn update_head(&mut self, commit: Hash) -> Result<()> {
        match &self.head.clone() {
            Reference::Hash(_) => self.set_head(Reference::Hash(commit)),
            Reference::Branch(b) => self.set_branch(b, commit),
        }
    }



    fn walk_worktree(&self) -> impl Iterator<Item = walkdir::Result<DirEntry>> {
        WalkDir::new(".")
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| !e.path().starts_with("./.gnew"))
            .filter(|e| match e {
                Ok(e) => !e.file_type().is_dir(),
                _ => true,
            })
    }

    pub fn checkout(&mut self, new_head: Reference, force: bool) -> Result<()> {
        let hash = self.resolve_reference(&new_head)?;

        /* first, we need to make sure that we are safe to switch to another commit,
         * which means there all the files in the dir are either Unmodified or Missing
         */

        /* if checkout is forced, skip the safe switch check */
        if !force {
            /* read commit by hash, get tree */
            let curr_tree = transport::read_commit(self.head_hash().unwrap())?.tree()?;
            let curr_status = self.status(&curr_tree)?;

            for f in curr_status {
                match f.1 {
                    FileStatus::Untracked
                    | FileStatus::Added
                    | FileStatus::Deleted
                    | FileStatus::Modified => return Err(CheckoutFailed),
                    FileStatus::Unmodified | FileStatus::Missing => continue,
                };
            }
        }

        /* next, we can do the actual checkout */

        /* read commit by hash, get tree */
        let tree = transport::read_commit(hash)?.tree()?;

        let status = self.status(&tree)?;

        for f in status {
            match f.1 {
                /* file was added, remove */
                FileStatus::Added => {
                    fs::remove_file(f.0)?;
                }
                /* file was deleted, copy over */
                FileStatus::Deleted => {
                    Repository::copy_objects_to_files(tree.files(), f.0)?;
                }
                /* file was modified, remove and copy over
                 * ideally, should do something fancy to modify existing file instead of copying, but oh well
                 */
                FileStatus::Modified => {
                    fs::remove_file(&f.0)?;
                    Repository::copy_objects_to_files(tree.files(), f.0)?;
                }
                /* file is the same, do nothing */
                FileStatus::Unmodified => continue,
                /* if checkout was forced, delete the untracked/missing file */
                FileStatus::Untracked | FileStatus::Missing => {
                    if force {
                        fs::remove_file(f.0)?
                    } else {
                        return Err(CheckoutFailed);
                    }
                }
            };
        }

        /* update tracklist on disc */
        let mut new_tracklist = Vec::new();
        for file in tree.files() {
            let File { path, .. } = file?;
            new_tracklist.push(path.to_str().unwrap().to_owned())
        }
        transport::write_tracklist(&new_tracklist)?;

        /* update HEAD */
        self.set_head(new_head)
    }

    fn copy_objects_to_files(files: FileIter, f: PathBuf) -> Result<()> {
        for file in files {
            let File { path, hash } = file?;
            if path == f {
                fs::create_dir_all(path.parent().unwrap())?;
                let blob: Blob = transport::read_blob(hash).map(|blob| blob.into())?;
                fs::write(path, blob.content)?;
                break;
            }
        }
        Ok(())
    }

    pub fn log(amount: u32) -> Result<Vec<Commit>> {
        let r = Repository::from_disc()?;

        let head_hash = match r.head_hash() {
            Ok(hash) => hash,
            Err(_) => return Ok(Vec::new()),
        };

        let mut count = 0;
        let mut commit_iter = transport::read_commit(head_hash)?.into_iter();
        let mut commit_vec: Vec<Commit> = Vec::new();

        while let Some(commit) = commit_iter.next() {
            if amount != 0 && count == amount {
                break;
            }

            commit_vec.push(commit?);
            count += 1;
        }

        Ok(commit_vec)
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Reference::Hash(h) => write!(f, "{}", h),
            Reference::Branch(b) => write!(f, "branch '{}'", b),
        }
    }
}

impl Commit {
    pub fn new(info: CommitInfo) -> Commit {
        Commit {
            hash: Hash::new(),
            tree: info.tree,
            parent: info.parent,
            author: info.author,
            time: info.time,
            msg: info.msg,
        }
    }

    pub fn hash(&self) -> Hash {
        self.hash
    }

    /// Set the hash to the hash of data.
    pub fn update_hash(&mut self, data: &[u8]) {
        self.hash.update(data)
    }

    pub fn tree_hash(&self) -> Hash {
        self.tree
    }

    pub fn tree(&self) -> Result<Tree> {
        transport::read_tree(self.tree)
    }

    pub fn parent_hash(&self) -> Option<Hash> {
        self.parent
    }

    pub fn parent(&self) -> Option<Result<Commit>> {
        self.parent.map(transport::read_commit)
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn into_iter(self) -> CommitIter {
        CommitIter { commit: Some(self) }
    }
}

impl fmt::Display for Commit {
    /// Formats a commit object in a format suitable for serialization.
    ///
    /// tree <tree hash>
    /// [parent <parent hash>]
    /// author <author name>
    /// time <timestamp>
    ///
    /// <commit message>
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "tree {}\n", self.tree)?;

        if let Some(parent) = self.parent {
            write!(f, "parent {}\n", parent)?;
        }
        write!(f, "author {}\n", self.author)?;
        write!(f, "time {}\n\n{}\n", self.time.timestamp_millis(), self.msg)
    }
}

#[derive(Debug)]
pub struct CommitIter {
    commit: Option<Commit>,
}

impl Iterator for CommitIter {
    type Item = Result<Commit>;

    fn next(&mut self) -> Option<Result<Commit>> {
        let out_commit = self.commit.take()?;
        match out_commit.parent() {
            Some(Ok(parent_commit)) => self.commit = Some(parent_commit),
            Some(Err(err)) => return Some(Err(err)),
            None => self.commit = None,
        }
        return Some(Ok(out_commit));
    }
}

impl Tree {
    pub fn new() -> Tree {
        Tree {
            hash: Hash::new(),
            entries: vec![],
        }
    }

    pub fn hash(&self) -> Hash {
        self.hash
    }

    /// Set the hash to the hash of data.
    pub fn update_hash(&mut self, data: &[u8]) {
        self.hash.update(data)
    }

    pub fn entries(&self) -> &[TreeEntry] {
        &self.entries
    }

    fn into_entries(self) -> vec::IntoIter<TreeEntry> {
        self.entries.into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Add a blob entry with the given hash and filename.
    pub fn add_blob(&mut self, hash: Hash, name: String) {
        self.entries.push(TreeEntry {
            kind: TreeEntryKind::Blob,
            hash,
            name,
        })
    }

    /// Add a tree entry with the given hash and filename.
    pub fn add_tree(&mut self, hash: Hash, name: String) {
        self.entries.push(TreeEntry {
            kind: TreeEntryKind::Tree,
            hash,
            name,
        })
    }

    /// Returns a file given its path in the tree.
    pub fn file<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let path = path.as_ref();
        let parts: Vec<_> = path.iter().collect();

        self.find_entry(&parts).and_then(|e| match e.kind() {
            TreeEntryKind::Tree => Err(FileNotFound),
            TreeEntryKind::Blob => Ok(File::new(path.into(), e.hash())),
        })
    }

    fn find_entry(&self, path: &[&OsStr]) -> Result<TreeEntry> {
        match path {
            [] => Err(FileNotFound),
            [f] => Ok(self.entry(f)?.to_owned()),
            [d, path @ ..] => Ok(self.dir(d)?.find_entry(path)?),
        }
    }

    fn dir(&self, name: &OsStr) -> Result<Tree> {
        self.entry(name).and_then(|e| match e.kind() {
            TreeEntryKind::Blob => Err(FileNotFound),
            TreeEntryKind::Tree => match transport::read_tree(e.hash()) {
                Err(ObjectNotFound) => Err(ObjectMissing),
                r => r,
            },
        })
    }

    fn entry(&self, name: &OsStr) -> Result<&TreeEntry> {
        self.entries
            .iter()
            .find(|&e| e.name() == name)
            .ok_or(FileNotFound)
    }

    /// Returns an iterator that recursively visits all files in the tree.
    pub fn files(&self) -> FileIter {
        FileIter {
            stack: vec![self.entries.clone().into_iter()],
            path: PathBuf::new(),
        }
    }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for entry in &self.entries {
            write!(f, "{} {}\t{}\n", entry.kind(), entry.hash(), entry.name())?
        }
        Ok(())
    }
}

impl TreeEntry {
    pub fn kind(&self) -> TreeEntryKind {
        self.kind
    }

    pub fn hash(&self) -> Hash {
        self.hash
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TreeEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TreeEntryKind::Blob => write!(f, "blob"),
            TreeEntryKind::Tree => write!(f, "tree"),
        }
    }
}

impl File {

    pub fn new(path: PathBuf, hash: Hash) -> File {
        File { path, hash }
    }

    pub fn contents(&self) -> Result<Vec<u8>> {
        transport::read_blob(self.hash).map(|blob| blob.into())
    }
}


/// An iterator over the files in a tree.
#[derive(Debug)]
pub struct FileIter {
    stack: Vec<vec::IntoIter<TreeEntry>>,
    path: PathBuf,
}

impl Iterator for FileIter {
    type Item = Result<File>;

    fn next(&mut self) -> Option<Result<File>> {
        loop {
            // infinite loop?
            assert!(self.stack.len() <= MAX_TREE_DEPTH);

            let entry = match self.stack.last_mut()?.next() {
                None => {
                    // end of current tree
                    self.stack.pop();
                    self.path.pop();
                    continue;
                }
                Some(entry) => entry,
            };
            match entry.kind() {
                TreeEntryKind::Blob => {
                    let path = self.path.join(entry.name());
                    return Some(Ok(File::new(path, entry.hash())));
                }
                TreeEntryKind::Tree => match transport::read_tree(entry.hash()) {
                    Err(ObjectNotFound) => return Some(Err(ObjectMissing)),
                    Err(err) => return Some(Err(err)),
                    Ok(tree) => {
                        self.stack.push(tree.into_entries().into_iter());
                        self.path.push(entry.name());
                        continue;
                    }
                },
            }
        }
    }
}

impl Blob {
    pub fn new(content: Vec<u8>) -> Blob {
        Blob {
            hash: Hash::new(),
            content,
        }
    }

    pub fn hash(&self) -> Hash {
        self.hash
    }

    /// Set the hash to the hash of data.
    pub fn update_hash(&mut self, data: &[u8]) {
        self.hash.update(data)
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

impl From<Blob> for Vec<u8> {
    fn from(blob: Blob) -> Vec<u8> {
        blob.content
    }
}

/// Computes the hash for a blob object with the contents of a file.
fn hash_file<P: AsRef<Path>>(path: P) -> Result<Hash> {
    let mut blob = Blob::new(fs::read(path)?);
    serialize_blob(&mut blob);
    Ok(blob.hash())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_repo_test() {
        let _a1 = Repository::create_empty();
    }
}
