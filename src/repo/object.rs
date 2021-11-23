use crate::storage::serialize::serialize_blob;
use crate::storage::transport::Error::*;
use crate::storage::transport::{self, read_lines_gen, write_commit, write_empty_repo, Result};
use chrono::{DateTime, TimeZone, Utc};
use sha1::{self, Sha1};
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::result;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec;
use walkdir::{self, DirEntry, WalkDir};
use std::str::FromStr;

const MAX_TREE_DEPTH: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hash(sha1::Digest);

#[derive(Debug)]
pub struct Repository {
    current_head: Option<Hash>,
    heads: HashMap<String, Option<Hash>>,
    pub tracklist: Vec<String>,
    detached: bool,
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
    tree: Hash,
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
            current_head: None,
            heads: HashMap::<String, Option<Hash>>::new(),
            tracklist: Vec::<String>::new(),
            detached: false,
        }
    }

    // creates empty repository and writes it to disc
    pub fn create_empty() -> Result<Repository> {
        let r = Repository::new();

        write_empty_repo()?;

        Ok(r)
    }

    /* TODO: read head, heads from disc */
    pub fn from_disc() -> Result<Repository> {
        let head = transport::read_curr_head()?;
        Ok(Repository {
            current_head: head.0,
            heads: transport::read_heads()?,
            tracklist: read_lines_gen(Path::new(".gnew/tracklist"))?,
            detached: head.1,
        })
    }

    pub fn heads(&self) -> &HashMap<String, Option<Hash>> {
        &self.heads
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
        let r = Repository::from_disc()?;

        let _newparent: Option<Hash> = match r.current_head {
            None => None,
            Some(ref c) => Some(*c),
        };
        let newtree: Result<Tree> = Repository::write_tree(&r);
        let _treehash = match newtree {
            Ok(c) => c.hash,
            Err(..) => Hash::new(),
        };
        let time = SystemTime::now().duration_since(UNIX_EPOCH);
        let currtime = match time {
            Ok(c) => c.as_millis(),
            Err(_) => 0,
        };
        let _date: DateTime<Utc> = Utc.timestamp(currtime as i64, 0);
        let mut user: String = "Temp user".to_string();
        let env_vars = env::vars();
        for (key, value) in env_vars.into_iter() {
            if key == "USER".to_string() {
                user = value;
            }
        }
        let _newcommit = CommitInfo {
            tree: _treehash,
            parent: _newparent,
            author: user,
            time: _date,
            msg: _cmsg,
        };
        let mut commit = Commit::new(_newcommit);
        let _resultcommit = write_commit(&mut commit);

        let branch_name = transport::current_branch()?;
        transport::update_head(branch_name, commit.hash())?;

        Ok(())
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

    pub fn checkout(init: String) -> Result<()> {
        let r = Repository::from_disc()?;

        let hash = match Hash::from_str(&init) {
            Ok(h) => h,
            _     => { match r.heads.get(&init).unwrap() {
                            Some(h) => *h,
                            /* branch has no commits - do nothing */
                            None => return Ok(())
                        }
                    }
        };

        /* read commit by hash, get tree hash, read tree by hash */
        let tree = transport::read_tree(transport::read_commit(hash)?.tree_hash())?;

        let status = r.status(&tree);
        let files = tree.files();

        Ok(())
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

    pub fn parent_hash(&self) -> Option<Hash> {
        self.parent
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
