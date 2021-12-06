use crate::storage::serialize::serialize_blob;
use crate::storage::transport;
use crate::wd::ui::{Error::*, Result};
use chrono::{DateTime, Utc};
use sha1::{self, Sha1};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::result;
use std::str;
use std::vec;

const MAX_TREE_DEPTH: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hash(sha1::Digest);

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub enum Change {
    Add(ChangeEntry),
    Remove(ChangeEntry),
    Modify(ChangeEntry, ChangeEntry),
}

#[derive(Debug, PartialEq)]
pub enum ChangeEntry {
    /// A stored file object.
    File(File),
    /// A working tree path.
    Path(PathBuf),
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

    pub fn into_common_ancestor(self, other: Commit) -> Result<Commit> {
        let mut ita = self.into_iter();
        let mut itb = other.into_iter();
        let mut amap = HashMap::new();
        let mut bmap = HashMap::new();

        loop {
            match ita.next().transpose()? {
                Some(c) if bmap.contains_key(&c.hash) => return Ok(c),
                Some(c) => {
                    amap.insert(c.hash, c);
                }
                None => (),
            }
            match itb.next().transpose()? {
                Some(c) if amap.contains_key(&c.hash) => return Ok(c),
                Some(c) => {
                    bmap.insert(c.hash, c);
                }
                None => (),
            }
        }
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

    /// Returns the changes between this tree and the provided one.
    pub fn diff(&self, to: &Tree) -> Result<Vec<Change>> {
        // This could be much faster if we pruned directories with equal hashes.
        let mut changes = vec![];
        let mut to_files = HashMap::new();

        for f in to.files() {
            let f = f?;
            to_files.insert(f.path.clone(), f);
        }
        for from in self.files() {
            let from = from?;
            let change = match to_files.remove(&from.path) {
                Some(to) if from.hash != to.hash => Change::new_modify(from, to),
                Some(_) => continue,
                None => Change::new_remove(from),
            };
            changes.push(change)
        }
        for to in to_files.into_values() {
            changes.push(Change::new_add(to))
        }
        Ok(changes)
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

impl Change {
    pub fn new_add<T: Into<ChangeEntry>>(new: T) -> Change {
        Change::Add(new.into())
    }

    pub fn new_remove<T: Into<ChangeEntry>>(old: T) -> Change {
        Change::Remove(old.into())
    }

    pub fn new_modify<T, U>(old: T, new: U) -> Change
    where
        T: Into<ChangeEntry>,
        U: Into<ChangeEntry>,
    {
        Change::Modify(old.into(), new.into())
    }

    /// Returns the path of the changed file.
    pub fn path(&self) -> &Path {
        match self {
            Change::Add(e) => e,
            Change::Remove(e) => e,
            Change::Modify(e, _) => e,
        }
        .path()
    }

    /// Returns the old and new contents.
    pub fn contents(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        Ok(match self {
            Change::Add(new) => (vec![], new.contents()?),
            Change::Remove(old) => (old.contents()?, vec![]),
            Change::Modify(old, new) => (old.contents()?, new.contents()?),
        })
    }
}

impl ChangeEntry {
    pub fn path(&self) -> &Path {
        match self {
            ChangeEntry::File(f) => &f.path,
            ChangeEntry::Path(p) => p,
        }
    }

    pub fn contents(&self) -> Result<Vec<u8>> {
        Ok(match self {
            ChangeEntry::File(f) => f.contents()?,
            ChangeEntry::Path(p) => fs::read(p)?,
        })
    }
}

impl From<File> for ChangeEntry {
    fn from(f: File) -> ChangeEntry {
        ChangeEntry::File(f)
    }
}

impl From<PathBuf> for ChangeEntry {
    fn from(p: PathBuf) -> ChangeEntry {
        ChangeEntry::Path(p)
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
pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<Hash> {
    let mut blob = Blob::new(fs::read(path)?);
    serialize_blob(&mut blob);
    Ok(blob.hash())
}
