use chrono::{DateTime, Utc};
use sha1::{self, Sha1};

#[derive(Debug, PartialEq)]
pub struct Hash(sha1::Digest);

#[derive(Debug)]
pub struct Repository {
    current_head: Option<Commit>,
    heads: Vec<Commit>,
    staging_area: Vec<String>
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
    blobs: Option<Vec<Blob>>
}

#[derive(Debug, PartialEq)]
pub struct Blob {
    hash: Hash,
    name: Option<String>,
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

impl Repository {
    pub fn new() -> Repository {
        Repository {
            current_head: None,
            heads: Vec::<Commit>::new(),
            staging_area: Vec::<String>::new()
        }
    }

    // creates empty repository and writes it to disc
    pub fn create_empty() -> Result<Repository, &'static str> {
        let r = Repository::new();
        // TODO: write to filesystem and read error value
        Ok(r)
    }
}

impl Blob {
    pub fn new() -> Blob {
        Blob {
            hash: Hash::new(),
            name: None,
            content: Vec::<u8>::new()
        }
    } 

    pub fn with_content(content: Vec<u8>) -> Blob {
        Blob {
            hash: Hash::new(),
            name: None,
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

    pub fn create_hash(&mut self) {
        self.hash.update(&self.content)
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }
}
