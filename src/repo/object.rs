use chrono::{DateTime, Utc};
use sha1::{self, Sha1};

#[derive(Debug, PartialEq)]
pub struct Hash(sha1::Digest);

impl Hash {
    pub fn new() -> Hash {
        Hash(Sha1::new().digest())
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0 = Sha1::from(data).digest()
    }
}

#[derive(Debug, PartialEq)]
pub struct Blob {
    hash: Hash,
    content: Vec<u8>,
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

pub struct Tree {
    hash: Hash,
    entries: Vec<TreeEntry>,
}

pub enum TreeEntry {
    /// A file.
    Blob { hash: Hash, fname: String },
    /// A directory.
    Tree { hash: Hash, fname: String },
}

pub struct Commit {
    hash: Hash,
    tree: Hash,
    parent: Option<Hash>,
    author: String,
    time: DateTime<Utc>,
    msg: String,
}
