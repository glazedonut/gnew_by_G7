use self::Error::*;
use super::serialize::{deserialize_blob, serialize_blob};
use crate::repo::object::{Blob, Commit, Hash, Tree};
use std::error;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ObjectNotFound,
    ObjectCorrupted,
    FileDoesNotExist,
    IoError(io::Error),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ObjectNotFound => write!(f, "object not found"),
            ObjectCorrupted => write!(f, "corrupted object"),
            FileDoesNotExist => write!(f, "file does not exist"),
            IoError(error) => write!(f, "IO error: {}", error),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        IoError(err)
    }
}

/// Creates and writes a blob object from the contents of a file.
pub fn write_blob<P: AsRef<Path>>(path: P) -> Result<Blob> {
    let mut blob = Blob::with_content(fs::read(path)?);
    let obj = serialize_blob(&mut blob);
    fs::write(object_path(blob.hash()), obj)?;
    Ok(blob)
}

/// Updates the hash of a tree object and writes it to storage.
pub fn write_tree(tree: Tree) -> Result<()> {
    todo!()
}

/// Updates the hash of a commit object and writes it to storage.
pub fn write_commit(commit: Commit) -> Result<()> {
    todo!()
}

/// Writes the DIR structure of an empty repo to disk
pub fn write_empty_repo() -> Result<()> {
    fs::create_dir_all(".gnew/objects")?;
    fs::create_dir(".gnew/heads")?;
    fs::write(".gnew/HEAD", "")?;
    fs::write(".gnew/tracklist", "")?;
    Ok(())
}

/* generic line filewriter. can be used to write to tracklist and HEAD files */
pub fn write_lines_gen<P: AsRef<Path>>(path: P, lines: &Vec<String>) -> Result<()> {
    let content = lines.join("\n");
    fs::write(path, content)?;
    Ok(())
}

/// Reads the blob object with the given hash from storage.
pub fn read_blob(hash: &Hash) -> Result<Blob> {
    let obj = fs::read(object_path(hash)).map_err(|err| match err.kind() {
        ErrorKind::NotFound => ObjectNotFound,
        _ => err.into(),
    })?;
    match deserialize_blob(&obj) {
        Some(blob) if blob.hash() == hash => Ok(blob),
        _ => Err(ObjectCorrupted),
    }
}

pub fn read_tree(hash: &Hash) -> Result<Tree> {
    todo!()
}

pub fn read_commit(hash: &Hash) -> Result<Commit> {
    todo!()
}

/* generic line filereader. can be used to read from tracklist and HEAD files */
pub fn read_lines_gen<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
    let fd = fs::File::open(path)?;
    let bufread = BufReader::new(fd);
    let mut out: Vec<String> = Vec::new();

    for line in bufread.lines() {
        out.push(line?);
    }

    Ok(out)
}

pub fn check_existence<P: AsRef<Path>>(files: &Vec<P>) -> Result<()> {
    for f in files {
        if !f.as_ref().exists() {
            return Err(FileDoesNotExist);
        }
    }
    Ok(())
}

fn object_path(hash: &Hash) -> PathBuf {
    let mut path = PathBuf::from(".gnew/objects");
    path.push(hash.to_string());
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_empty_repo() {
        let a1 = write_empty_repo();
    }

    // for now you have to create .gnew/objects before running these tests
    // TODO: run them in a test repo

    #[test]
    fn read_write_blob() {
        fs::write("foo.txt", b"test content").unwrap();
        let b1 = write_blob("foo.txt").unwrap();
        let b2 = read_blob(b1.hash()).unwrap();
        assert_eq!(b1, b2);
    }
}
