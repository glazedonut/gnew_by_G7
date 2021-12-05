use self::Error::*;
use super::serialize::*;
use crate::repo::object::{Blob, Commit, Hash, Reference, Tree};
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::result;
use walkdir::WalkDir;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BranchExists,
    CheckoutFailed,
    FileNotFound,
    IoError(io::Error),
    ObjectCorrupted,
    ObjectMissing,
    ObjectNotFound,
    ReferenceNotFound,
    NoRepository,
    PushFailed,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BranchExists => write!(f, "branch already exists"),
            CheckoutFailed => write!(f, "commit or remove changes first"),
            FileNotFound => write!(f, "file not found"),
            IoError(error) => write!(f, "IO error: {}", error),
            ObjectCorrupted => write!(f, "corrupted object"),
            ObjectMissing => write!(f, "missing object"),
            ObjectNotFound => write!(f, "object not found"),
            ReferenceNotFound => write!(f, "reference not found"),
            NoRepository => write!(f, "no repository at file path"),
            PushFailed => write!(f, "local and remote repositories differ. pull first"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        IoError(err)
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Error {
        IoError(err.into())
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
    fs::write(".gnew/HEAD", "ref: main\n")?;
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

pub fn check_repo_exists<P: AsRef<Path>>(repo: P) -> Result<PathBuf> {
    let gnew = Path::new(".gnew/");
    let full_path = repo.as_ref().join(gnew);
    if full_path.exists() {
        Ok(full_path)
    } else {
        Err(NoRepository)
    }
}

fn object_path(hash: Hash) -> PathBuf {
    let mut path = PathBuf::from(".gnew/objects");
    path.push(hash.to_string());
    path
}

pub fn read_tracklist<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
    let path = path.as_ref().join(PathBuf::from(".gnew/tracklist"));
    read_lines_gen(path)
}

pub fn write_tracklist<P: AsRef<Path>>(path: P, lines: &Vec<String>) -> Result<()> {
    let path = path.as_ref().join(PathBuf::from(".gnew/tracklist"));
    write_lines_gen(path, lines)
}

pub fn write_head<P: AsRef<Path>>(path: P, r: &Reference) -> Result<()> {
    let mut f = File::create(path.as_ref().join(Path::new(".gnew/HEAD")))?;
    match r {
        Reference::Hash(h) => writeln!(f, "{}", h),
        Reference::Branch(b) => writeln!(f, "ref: {}", b),
    }?;
    Ok(())
}

pub fn read_head<P: AsRef<Path>>(path: P) -> Result<Reference> {
    let head = fs::read_to_string(path.as_ref().join(Path::new(".gnew/HEAD")))?;
    let head = head.trim();

    Ok(match head.strip_prefix("ref: ") {
        Some(b) => Reference::Branch(b.to_owned()),
        None => Reference::Hash(head.parse().or(Err(ObjectCorrupted))?),
    })
}

pub fn write_branch<P: AsRef<Path>>(path: P, name: &str, commit: Hash) -> Result<()> {
    let mut f = File::create(path.as_ref().join(Path::new(".gnew/heads").join(name)))?;
    writeln!(f, "{}", commit)?;
    Ok(())
}

pub fn read_branches<P: AsRef<Path>>(r_path: P) -> Result<HashMap<String, Hash>> {
    let mut branches = HashMap::new();

    for f in WalkDir::new(r_path.as_ref().join(Path::new(".gnew/heads"))) {
        let f = f?;
        if !f.file_type().is_file() {
            continue;
        }
        let path = f.path();
        let name = path
            .strip_prefix(r_path.as_ref().to_str().unwrap())
            .unwrap()
            .to_str()
            .unwrap()
            .strip_prefix(".gnew/heads/")
            .unwrap();
        let hash = fs::read_to_string(path)?
            .trim()
            .parse()
            .or(Err(ObjectCorrupted))?;

        branches.insert(name.to_owned(), hash);
    }
    Ok(branches)
}

pub fn get_objects<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
    let objects_dir = path.as_ref().join(Path::new("objects"));
    let objects = fs::read_dir(&objects_dir)?;
    Ok(objects
        .map(|x| {
            x.unwrap()
                .path()
                .strip_prefix(&objects_dir)
                .unwrap()
                .to_path_buf()
        })
        .collect::<Vec<PathBuf>>())
}

pub fn copy_objects<P: AsRef<Path>>(from: P, to: P, objects: Vec<PathBuf>) -> Result<()> {
    let from_path = from.as_ref().join(Path::new("objects/"));
    let to_path = to.as_ref().join(Path::new("objects/"));

    for o in objects {
        fs::copy(from_path.join(&o), to_path.join(&o))?;
    }

    Ok(())
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
