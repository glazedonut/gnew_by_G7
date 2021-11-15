use crate::repo::object::{Blob, Commit, Tree};

/// Serializes a blob object and updates its hash.
pub fn serialize_blob(blob: &mut Blob) -> Vec<u8> {
    // Blob format: `blob<NUL><content>`
    let obj = [b"blob\0".as_ref(), blob.content()].concat();
    blob.update_hash(&obj);
    obj
}

/// Serializes a tree object and updates its hash.
pub fn serialize_tree(tree: &mut Tree) -> Vec<u8> {
    todo!()
}

/// Serializes a commit object and updates its hash.
pub fn serialize_commit(commit: &mut Commit) -> Vec<u8> {
    todo!()
}

/// Deserializes a blob object.
/// Returns None if obj is not a valid blob object.
pub fn deserialize_blob(obj: &[u8]) -> Option<Blob> {
    obj.strip_prefix(b"blob\0").map(|content| {
        let mut blob = Blob::with_content(content.to_vec());
        blob.update_hash(obj);
        blob
    })
}

/// Deserializes a tree object.
/// Returns None if obj is not a valid tree object.
pub fn deserialize_tree(obj: &[u8]) -> Option<Tree> {
    todo!()
}

/// Deserializes a commit object.
/// Returns None if obj is not a valid commit object.
pub fn deserialize_commit(obj: &[u8]) -> Option<Commit> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_blob() {
        let mut b1 = Blob::with_content(b"hello world".to_vec());
        let b2 = deserialize_blob(&serialize_blob(&mut b1)).unwrap();
        assert_eq!(b1, b2);
    }
}
