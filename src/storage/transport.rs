use super::serialize;
use crate::repo::object::{Blob, Commit, Hash, Tree};
use std::io;
use std::path::Path;

/// Creates and writes a blob object from the contents of a file.
/// Returns the hash or error.
pub fn write_blob(path: &Path) -> io::Result<Hash> {
    // create blob from file contents, serialize blob, write to .gnew/objects/hash
    todo!()
}

pub fn write_tree(tree: Tree) -> io::Result<Hash> {
    todo!()
}

pub fn write_commit(commit: Commit) -> io::Result<Hash> {
    todo!()
}

/// Reads the blob object with the given hash from storage.
pub fn read_blob(hash: Hash) -> Result<Blob, &'static str> {
    // deserialize blob from .gnew/objects/hash and verify that the blob has the requested hash.
    todo!()
}

pub fn read_tree(hash: Hash) -> Result<Tree, &'static str> {
    todo!()
}

pub fn read_commit(hash: Hash) -> Result<Commit, &'static str> {
    todo!()
}
