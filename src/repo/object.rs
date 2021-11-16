use crate::storage::transport::{write_empty_repo, write_lines_gen, read_lines_gen, check_existence, Error};
use chrono::{DateTime, Utc};
use sha1::{self, Sha1};
use std::fmt;
use std::str;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub struct Hash(sha1::Digest);

#[derive(Debug)]
pub struct Repository {
    current_head: Option<Commit>,
    heads: Vec<Commit>,
    tracklist: Vec<String>,
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
            tracklist: read_lines_gen(Path::new(".gnew/tracklist"))?
        })
    }

    /* adds the specfied files to the tracklist on disc */
    pub fn add_to_tracklist<P: AsRef<Path>>(files: &Vec<P>) -> Result<(), Error> {
        /* check that all the specified files exist */
        check_existence(files)?;

        /* read current state of repository from disc */
        let mut r = Repository::from_disc()?;

        /* if file isn't tracked already, add it to tracklist
         * note that this adds directories just like files
         * during commit, dirs have to be added recursively
         */
        for f in files {
            let s = f.as_ref().to_str().unwrap().to_string();
            if !r.tracklist.contains(&s) {
                r.tracklist.push(s);
            }
        }

        /* write new tracklist to .gnew/tracklist */
        write_lines_gen(Path::new(".gnew/tracklist"), &r.tracklist)?;

        Ok(())
    }
}

impl Blob {
    pub fn new() -> Blob {
        Blob {
            hash: Hash::new(),
            name: None,
            content: Vec::<u8>::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_repo_test() {
        let a1 = Repository::create_empty();
    }
}
