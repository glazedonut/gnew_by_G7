use crate::storage::transport::{self, read_lines_gen, write_empty_repo, Result, write_commit};

use chrono::{DateTime, Utc, TimeZone};
use sha1::{self, Sha1};
use std::fmt;
use std::fs;
use std::path::Path;
use std::result;
use std::env;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hash(sha1::Digest);

#[derive(Debug)]
pub struct Repository {
    current_head: Option<Hash>,
    heads: Vec<(String, Hash)>,
    pub tracklist: Vec<String>,
    detached: bool,
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

#[derive(Debug, PartialEq)]
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
            heads: Vec::<(String, Hash)>::new(),
            tracklist: Vec::<String>::new(),
            detached: false
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
            current_head: Some(head.0),
            heads: transport::read_heads()?,
            tracklist: read_lines_gen(Path::new(".gnew/tracklist"))?,
            detached: head.1
        })
    }

    pub fn heads(&self) -> &Vec<(String, Hash)> {
        &self.heads
    }

    /// Checks if a file is tracked.
    pub fn is_tracked(&self, path: &Path) -> bool {
        self.tracklist.contains(&path.to_str().unwrap().to_owned())
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
        _cmsg=match commitmsg {
            Some(ref c) =>c.to_string(),
            None => "".to_string(),
        };
        let r = Repository::from_disc()?;


        let _newparent:Option<Hash> =match r.current_head{
            None => {None}
            Some(ref c) => {Some(*c)}
        };
        let newtree:Result<Tree>=Repository::write_tree(&r);
        let _treehash= match newtree{
            Ok(c) => {c.hash}
            Err(..) => {Hash::new()}
        };
        let time=SystemTime::now().duration_since(UNIX_EPOCH);
        let currtime = match time{
            Ok(c) => {c.as_millis()}
            Err(_) => {0}
        };
       let _date:DateTime<Utc>=Utc.timestamp(currtime as i64, 0);
        let mut user:String=String::new();
        let env_vars = env::vars();
        for(key, value) in env_vars.into_iter() {
            if key=="USER".to_string() {
                user=value;
            }
        }
        let _newcommit=CommitInfo {
            tree: _treehash,
            parent: _newparent,
            author: user,
            time: _date,
            msg: _cmsg,
        };
        let mut commit =Commit::new(_newcommit);
        let _resultcommit= write_commit(&mut commit);



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

    pub fn tree(&self) -> Hash {
        self.tree
    }

    pub fn parent(&self) -> Option<Hash> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_repo_test() {
        let _a1 = Repository::create_empty();
    }
}
