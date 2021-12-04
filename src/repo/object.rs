use crate::storage::serialize::serialize_blob;
use crate::storage::transport::Error::*;
use crate::storage::transport::{self, Result};
use chrono::{DateTime, Utc};
use sha1::{self, Sha1};
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::result;
use std::str;
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
    worktree: PathBuf,
    storage_dir: PathBuf,
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

impl FileStatus {
    pub fn code(&self) -> char {
        match self {
            FileStatus::Untracked => '?',
            FileStatus::Unmodified => ' ',
            FileStatus::Modified => 'M',
            FileStatus::Added => 'A',
            FileStatus::Deleted => 'R',
            FileStatus::Missing => '!',
        }
    }
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

impl Repository {
    /// Creates an empty repository in the current directory.
    pub fn init() -> Result<Repository> {
        let worktree = fs::canonicalize(".")?;
        let storage_dir = worktree.join(".gnew");
        transport::write_empty_repo()?;

        Ok(Repository {
            head: Reference::Branch("main".to_owned()),
            branches: HashMap::new(),
            tracklist: Vec::<String>::new(),
            worktree,
            storage_dir,
        })
    }

    /// Opens a repository in the current directory.
    pub fn open() -> Result<Repository> {
        let worktree = fs::canonicalize(".")?;
        let storage_dir = transport::check_repo_exists(&worktree)?;

        Ok(Repository {
            head: transport::read_head(&worktree)?,
            branches: transport::read_branches(&worktree)?,
            tracklist: transport::read_tracklist(&worktree)?,
            worktree,
            storage_dir,
        })
    }

    pub fn open_remote<P: AsRef<Path>>(remote: P) -> Result<Repository> {
        let worktree = fs::canonicalize(remote)?;
        let storage_dir = transport::check_repo_exists(&worktree)?;

        Ok(Repository {
            head: transport::read_head(&worktree)?,
            branches: transport::read_branches(&worktree)?,
            tracklist: transport::read_tracklist(&worktree)?,
            worktree,
            storage_dir,
        })
    }

    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
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
    /// The path can be absolute or relative to the working tree.
    pub fn is_tracked(&self, path: &Path) -> bool {
        let path = path.strip_prefix(&self.worktree).unwrap_or(path);
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
        for f in self.walk_worktree(Path::new(".")) {
            let f = f?;
            let path = f.path();
            let rpath = path.strip_prefix(&self.worktree).unwrap();

            let fstatus = match (head_files.remove(rpath), self.is_tracked(path)) {
                (None, true) => FileStatus::Added,
                (None, false) => FileStatus::Untracked,
                (Some(hash), true) => {
                    if hash_file(path)? == hash {
                        FileStatus::Unmodified
                    } else {
                        FileStatus::Modified
                    }
                }
                (Some(_), false) => FileStatus::Deleted,
            };
            status.insert(rpath.to_owned(), fstatus);
        }
        for path in head_files.into_keys() {
            let fstatus = if self.is_tracked(&path) {
                FileStatus::Missing
            } else {
                FileStatus::Deleted
            };
            status.insert(path, fstatus);
        }
        Ok(status)
    }

    /// Writes a tree object from the working directory.
    pub fn write_tree(&self) -> Result<Tree> {
        let mut tree = self.write_tree_rec(&self.worktree)?;
        transport::write_tree(&mut tree)?;
        Ok(tree)
    }

    fn write_tree_rec(&self, dir: &Path) -> Result<Tree> {
        let mut tree = Tree::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.starts_with(&self.storage_dir) {
                continue;
            }
            let fname = entry.file_name().to_str().unwrap().to_owned();

            if entry.file_type()?.is_dir() {
                let mut subtree = self.write_tree_rec(&path)?;
                if !subtree.is_empty() {
                    transport::write_tree(&mut subtree)?;
                    tree.add_tree(subtree.hash(), fname)
                }
            } else if self.is_tracked(&path) {
                tree.add_blob(transport::write_blob(path)?.hash(), fname)
            }
        }
        Ok(tree)
    }

    pub fn commit(&mut self, msg: String) -> Result<Commit> {
        let tree = self.write_tree()?;
        let user = env::var("USER").unwrap_or_else(|_| "noname".to_owned());

        let mut commit = Commit::new(CommitInfo {
            tree: tree.hash(),
            parent: self.head_hash().ok(),
            author: user,
            time: Utc::now(),
            msg,
        });

        transport::write_commit(&mut commit)?;
        self.update_head(commit.hash())?;
        Ok(commit)
    }

    fn update_head(&mut self, commit: Hash) -> Result<()> {
        match &self.head.clone() {
            Reference::Hash(_) => self.set_head(Reference::Hash(commit)),
            Reference::Branch(b) => self.set_branch(b, commit),
        }
    }

    pub fn checkout(&mut self, new_head: Reference, force: bool) -> Result<()> {
        let hash = self.resolve_reference(&new_head)?;

        /* first, we need to make sure that we are safe to switch to another commit,
         * which means there all the files in the dir are either Unmodified or Missing
         */

        /* if checkout is forced, skip the safe switch check */
        if !force {
            self.check_safe_switch()?
        }

        /* next, we can do the actual checkout */

        /* read commit by hash, get tree */
        let tree = transport::read_commit(hash)?.tree()?;
        let status = self.status(&tree)?;
        let mut tree_files = HashMap::new();

        for f in tree.files() {
            let File { path, hash } = f?;
            tree_files.insert(path, hash);
        }

        for f in status {
            match f.1 {
                /* file was added, remove */
                FileStatus::Added => {
                    fs::remove_file(f.0)?;
                }
                /* file was deleted, copy over */
                FileStatus::Deleted => {
                    Repository::copy_objects_to_files(&tree_files, f.0)?;
                }
                /* file was modified, remove and copy over */
                FileStatus::Modified => {
                    fs::remove_file(&f.0)?;
                    Repository::copy_objects_to_files(&tree_files, f.0)?;
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
        for file in tree_files {
            new_tracklist.push(file.0.to_str().unwrap().to_owned())
        }
        transport::write_tracklist(&new_tracklist)?;

        /* update HEAD */
        transport::write_head(&new_head)?;
        self.set_head(new_head)
    }

    fn copy_objects_to_files(files: &HashMap<PathBuf, Hash>, f: PathBuf) -> Result<()> {
        for file in files {
            if *(file.0) == f {
                fs::create_dir_all(file.0.parent().unwrap())?;
                let blob: Blob = transport::read_blob(*file.1).map(|blob| blob.into())?;
                fs::write(file.0, blob.content)?;
                break;
            }
        }
        Ok(())
    }

    pub fn log(&self, amount: u32) -> Result<Vec<Commit>> {
        let head_hash = match self.head_hash() {
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

    pub fn add<P: AsRef<Path>>(&mut self, files: &Vec<P>) -> Result<()> {
        transport::check_existence(files)?;

        for file in files {
            let file = fs::canonicalize(file)?;
            let f: &Path = file.strip_prefix(&self.worktree).unwrap();
            let md = fs::metadata(f).unwrap();

            if md.is_file() {
                let p = f.to_str().unwrap().to_string();
                if !self.tracklist.contains(&p) {
                    self.tracklist.push(p);
                }
            } else if md.is_dir() {
                let mut paths: Vec<String> = Vec::new();

                for entry in self.walk_worktree(f.as_ref()) {
                    let entry = entry?;
                    let p = entry.path().strip_prefix(&self.worktree).unwrap();
                    paths.push(p.to_str().unwrap().to_string());
                }

                for p in paths {
                    if !self.tracklist.contains(&p) {
                        self.tracklist.push(p);
                    }
                }
            }
        }

        transport::write_tracklist(&self.tracklist)?;

        Ok(())
    }

    pub fn remove<P: AsRef<Path>>(&mut self, files: &Vec<P>) -> Result<()> {
        transport::check_existence(files)?;

        for f in files {
            let f = fs::canonicalize(f)?;
            let p = f.strip_prefix(&self.worktree).unwrap();
            let md = fs::metadata(p).unwrap();
            let mut prefix = p.to_str().unwrap().to_string();

            if md.is_file() {
                self.tracklist.retain(|x| !(*x == prefix));
            } else if md.is_dir() {
                let mut tmp = [0u8; 4];
                prefix = prefix + '/'.encode_utf8(&mut tmp);

                self.tracklist.retain(|x| !(*x).starts_with(&prefix));
            }
        }

        transport::write_tracklist(&self.tracklist)?;

        Ok(())
    }

    fn walk_worktree(&self, path: &Path) -> impl Iterator<Item = walkdir::Result<DirEntry>> + '_ {
        WalkDir::new(self.worktree.join(path))
            .into_iter()
            .filter_entry(|e| !e.path().starts_with(&self.storage_dir))
            .filter(|e| match e {
                Ok(e) => !e.file_type().is_dir(),
                _ => true,
            })
    }

    pub fn pull<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let remote = Repository::open_remote(path)?;

        let mut remote_objects = transport::get_objects(&remote.storage_dir)?;
        let local_objects = transport::get_objects(&self.storage_dir)?;

        for remote_branch in remote.branches() {
            match self.branches.get_mut(remote_branch.0) {
                /* a local branch with the same name exists */
                Some(local_hash) => {
                    if remote_objects.contains(&PathBuf::from(local_hash.to_string())) {
                        /* if the last commit of the branch is stored in remote repo,
                         * can skip "fast-forward" merge by just moving the branch hash
                         */
                        *local_hash = *remote_branch.1;
                    } else {
                        /* have to merge */
                        println!("merge");
                        todo!();
                    }
                }
                /* no local branch by the name, create new */
                None => {
                    self.branches
                        .insert((*remote_branch.0).to_string(), *remote_branch.1);
                }
            };
        }

        /* remove any objects that already exist */
        remote_objects.retain(|x| !local_objects.contains(x));
        /* copy objects from remote to local */
        transport::copy_objects(&remote.storage_dir, &self.storage_dir, remote_objects)?;

        /* update local branches on disk */
        for b in &self.branches {
            transport::write_branch(&b.0, *b.1)?;
        }

        /* switch to latest version of branch head */
        self.checkout(self.head.clone(), true)?;

        Ok(())
    }

    fn check_safe_switch(&self) -> Result<()> {
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

        Ok(())
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
        let _a1 = Repository::init();
    }
    #[test]
    fn add_test() {
        let mut path = env::current_dir().unwrap_or(PathBuf::new());
        path.push("object.rs");
        let mut r = Repository::open().unwrap();
        r.add(&vec![path]);
    }
    #[test]
    fn commit_test() {
        let mut r = Repository::open().unwrap();
        r.commit("test commit".to_string());
    }
}
