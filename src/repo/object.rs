use crate::storage::transport::{read_lines_gen, write_empty_repo, Error};
use chrono::{DateTime, Utc};
use sha1::{self, Sha1};
use std::fmt;
use std::path::Path;
use std::str;

#[derive(Debug, PartialEq)]
pub struct Hash(sha1::Digest);

#[derive(Debug)]
pub struct Repository {
    current_head: Option<Commit>,
    heads: Vec<Commit>,
    pub tracklist: Vec<String>,
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

#[derive(Debug, PartialEq)]
pub struct Tree {
    hash: Hash,
    name: Option<String>,
    trees: Option<Vec<Tree>>,
    blobs: Option<Vec<Blob>>,
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

    fn from_str(s: &str) -> Result<Hash, sha1::DigestParseError> {
        Ok(Hash(s.parse()?))
    }
}

impl Repository {
    pub fn new() -> Repository {
        Repository {
            current_head: None,
            heads: Vec::<Commit>::new(),
            tracklist: Vec::<String>::new(),
        }
    }

    // creates empty repository and writes it to disc
    pub fn create_empty() -> Result<Repository, Error> {
        let r = Repository::new();

        write_empty_repo()?;

        Ok(r)
    }

    /* TODO: read head, heads from disc */
    pub fn from_disc() -> Result<Repository, Error> {
        Ok(Repository {
            current_head: None,
            heads: Vec::<Commit>::new(),
            tracklist: read_lines_gen(Path::new(".gnew/tracklist"))?,
        })
    }
}

impl Blob {
    pub fn new(content: Vec<u8>) -> Blob {
        Blob {
            hash: Hash::new(),
            content,
        }
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
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
        let a1 = Repository::create_empty();
    }
}
