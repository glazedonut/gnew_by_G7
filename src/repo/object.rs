use chrono::{DateTime, Utc};
use sha1;

pub struct Hash(sha1::Digest);

pub struct Blob {
    hash: Hash,
    content: Vec<u8>,
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
