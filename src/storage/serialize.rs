use crate::repo::object::{Blob, Commit, Tree};

// Length of the string representation of a hash.
const HASH_LENGTH: usize = 40;

/// Serializes a blob object and updates its hash.
pub fn serialize_blob(blob: &mut Blob) -> Vec<u8> {
    // blob format: `blob<NUL><content>`
    let obj = [b"blob\0".as_ref(), blob.content()].concat();
    blob.update_hash(&obj);
    obj
}

/// Serializes a tree object and updates its hash.
pub fn serialize_tree(tree: &mut Tree) -> Vec<u8> {
    // tree format: `tree<NUL><entries>`
    // entry format: `<type> <filename><NUL><hash>`
    let mut entries: Vec<Vec<u8>> = tree
        .entries()
        .iter()
        .map(|e| format!("{} {}\0{}", e.kind(), e.name(), e.hash()).into_bytes())
        .collect();

    // sort by filename
    entries.sort_unstable_by(|x, y| x[5..].cmp(&y[5..]));

    let obj = [b"tree\0".to_vec(), entries.concat()].concat();
    tree.update_hash(&obj);
    obj
}

/// Serializes a commit object and updates its hash.
pub fn serialize_commit(commit: &mut Commit) -> Vec<u8> {
    todo!()
}

/// Deserializes a blob object.
/// Returns None if obj is not a valid blob object.
pub fn deserialize_blob(obj: &[u8]) -> Option<Blob> {
    obj.strip_prefix(b"blob\0").map(|content| {
        let mut blob = Blob::new(content.to_vec());
        blob.update_hash(obj);
        blob
    })
}

/// Deserializes a tree object.
/// Returns None if obj is not a valid tree object.
pub fn deserialize_tree(obj: &[u8]) -> Option<Tree> {
    obj.strip_prefix(b"tree\0").and_then(|entries| {
        let mut tree = deserialize_tree_entries(entries)?;
        tree.update_hash(obj);
        Some(tree)
    })
}

fn deserialize_tree_entries(obj: &[u8]) -> Option<Tree> {
    let mut tree = Tree::new();
    let mut it = obj.split(|&b| b == b'\0');
    let first = match it.next() {
        None => return Some(tree),
        Some(x) => x,
    };
    it.try_fold(first, |a, b| {
        let (kind, name) = (a.get(..4)?, a.get(5..)?);
        let (hash, next) = (b.get(..HASH_LENGTH)?, b.get(HASH_LENGTH..)?);
        let name = String::from_utf8(name.to_vec()).ok()?;
        let hash = String::from_utf8(hash.to_vec()).ok()?;
        let hash = hash.parse().ok()?;
        match kind {
            b"blob" => tree.add_blob(hash, name),
            b"tree" => tree.add_tree(hash, name),
            _ => return None,
        };
        Some(next)
    })
    .and_then(|rest| if rest.is_empty() { Some(tree) } else { None })
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
        let mut b1 = Blob::new(b"hello world".to_vec());

        let obj = serialize_blob(&mut b1);
        assert_eq!(obj, b"blob\0hello world");

        let b2 = deserialize_blob(&obj).unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn serde_tree() {
        let mut t1 = Tree::new();
        let mut bar = Blob::new(b"bar".to_vec());
        let mut foo = Blob::new(b"foo".to_vec());
        serialize_blob(&mut foo);
        serialize_blob(&mut bar);
        t1.add_blob(bar.hash(), "bar.txt".to_owned());
        t1.add_blob(foo.hash(), "foo.txt".to_owned());

        let obj = serialize_tree(&mut t1);
        let expected = format!(
            "tree\0blob bar.txt\0{}blob foo.txt\0{}",
            bar.hash(),
            foo.hash()
        );
        assert_eq!(obj, expected.into_bytes());

        let t2 = deserialize_tree(&obj).unwrap();
        assert_eq!(t1, t2);
    }
}
