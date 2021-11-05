use crate::repo::object::{Blob, Commit, Tree};

/// Serializes a blob object and updates its hash.
pub fn serialize_blob(blob: Blob) -> Vec<u8> {
    todo!()
}

/// Serializes a tree object and updates its hash.
pub fn serialize_tree(tree: Tree) -> Vec<u8> {
    todo!()
}

/// Serializes a commit object and updates its hash.
pub fn serialize_commit(commit: Commit) -> Vec<u8> {
    todo!()
}

/// Deserializes a blob object.
/// Returns None if obj is not a valid blob object.
pub fn deserialize_blob(obj: Vec<u8>) -> Option<Blob> {
    todo!()
}

/// Deserializes a tree object.
/// Returns None if obj is not a valid tree object.
pub fn deserialize_tree(obj: Vec<u8>) -> Option<Tree> {
    todo!()
}

/// Deserializes a commit object.
/// Returns None if obj is not a valid commit object.
pub fn deserialize_commit(obj: Vec<u8>) -> Option<Commit> {
    todo!()
}
