use self::Error::*;
use super::serialize::*;
use crate::repo::object::{Blob, Commit, Hash, Tree};
use std::error;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::result;
use std::str::FromStr;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ObjectNotFound,
    ObjectMissing,
    ObjectCorrupted,
    FileNotFound,
    IoError(io::Error),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ObjectNotFound => write!(f, "object not found"),
            ObjectMissing => write!(f, "missing object"),
            ObjectCorrupted => write!(f, "corrupted object"),
            FileNotFound => write!(f, "file not found"),
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
    let mut blob = Blob::new(fs::read(path)?);
    let obj = serialize_blob(&mut blob);
    write_object(blob.hash(), &obj)?;
    Ok(blob)
}

/// Updates the hash of a tree object and writes it to storage.
pub fn write_tree(tree: &mut Tree) -> Result<()> {
    let obj = serialize_tree(tree);
    write_object(tree.hash(), &obj)
}

/// Updates the hash of a commit object and writes it to storage.
pub fn write_commit(commit: &mut Commit) -> Result<()> {
    let obj = serialize_commit(commit);
    write_object(commit.hash(), &obj)
}

fn write_object(hash: Hash, obj: &[u8]) -> Result<()> {
    let path = object_path(hash);
    if !path.exists() {
        fs::write(path, obj)?;
    }
    Ok(())
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
pub fn read_blob(hash: Hash) -> Result<Blob> {
    match deserialize_blob(&read_object(hash)?) {
        Some(blob) if blob.hash() == hash => Ok(blob),
        _ => Err(ObjectCorrupted),
    }
}

/// Reads the tree object with the given hash from storage.
pub fn read_tree(hash: Hash) -> Result<Tree> {
    match deserialize_tree(&read_object(hash)?) {
        Some(tree) if tree.hash() == hash => Ok(tree),
        _ => Err(ObjectCorrupted),
    }
}

/// Reads the commit object with the given hash from storage.
pub fn read_commit(hash: Hash) -> Result<Commit> {
    match deserialize_commit(&read_object(hash)?) {
        Some(commit) if commit.hash() == hash => Ok(commit),
        _ => Err(ObjectCorrupted),
    }
}

fn read_object(hash: Hash) -> Result<Vec<u8>> {
    fs::read(object_path(hash)).map_err(|err| match err.kind() {
        ErrorKind::NotFound => ObjectNotFound,
        _ => err.into(),
    })
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
            return Err(FileNotFound);
        }
    }
    Ok(())
}

fn object_path(hash: Hash) -> PathBuf {
    let mut path = PathBuf::from(".gnew/objects");
    path.push(hash.to_string());
    path
}

pub fn read_tracklist() -> Result<Vec<String>> {
    let path = PathBuf::from(".gnew/tracklist");
    read_lines_gen(path)
}

pub fn write_tracklist(lines: &Vec<String>) -> Result<()> {
    let path = PathBuf::from(".gnew/tracklist");
    write_lines_gen(path, lines)
}

pub fn read_curr_head() -> Result<(Hash, bool)> {
    let branch_name = read_lines_gen(PathBuf::from(".gnew/HEAD"))?;
    if branch_name.len() == 2 {
        match Hash::from_str(&branch_name.get(1).unwrap()) {
            Ok(h) => Ok((h, true)),
            _ => Err(ObjectCorrupted),
        }
    } else if branch_name.len() == 1 {
        let commit_hash = read_lines_gen(PathBuf::from(
            ".gnew/heads/".to_owned() + branch_name.get(0).unwrap(),
        ))?;

        match Hash::from_str(&commit_hash.get(0).unwrap()) {
            Ok(h) => Ok((h, false)),
            _ => Err(ObjectCorrupted),
        }
    } else {
        Err(ObjectCorrupted)
    }
}

pub fn read_heads() -> Result<Vec<(String, Hash)>> {
    let paths = fs::read_dir(".gnew/heads").unwrap();
    let mut heads: Vec<(String, Hash)> = Vec::new();

    for p in paths {
        let path = p.as_ref().unwrap().path();
        let lines = read_lines_gen(&path)?;
        let branch_name = path.to_str().unwrap().split("/");

        match Hash::from_str(&lines.get(0).unwrap()) {
            Ok(h) => heads.push((branch_name.last().unwrap().to_string(), h)),
            _ => return Err(ObjectCorrupted),
        }
    }

    Ok(heads)
}

pub fn change_curr_head(branch_name: String) -> Result<()> {
    write_lines_gen(PathBuf::from(".gnew/HEAD"), &vec![branch_name])
}

pub fn write_head(branch_name: String, hash: Hash) -> Result<()> {
    write_lines_gen(
        PathBuf::from(".gnew/".to_owned() + &branch_name),
        &vec![hash.to_string()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_empty_repo() {
        let _a1 = write_empty_repo();
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
