use super::serialize;
use crate::repo::object::{Blob, Commit, Hash, Tree};
use std::io;

/// Updates the hash of a blob object and writes it to storage.
/// Returns the hash or error.
pub fn write_blob(blob: Blob) -> io::Result<Hash> {
    // Call serialize_blob and write the byte vector to a file.
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
    // Read file and pass the contents to deserialize_blob.
    // Set the hash on the returned object.
    todo!()
}

pub fn read_tree(hash: Hash) -> Result<Tree, &'static str> {
    todo!()
}

pub fn read_commit(hash: Hash) -> Result<Commit, &'static str> {
    todo!()
}
