use crate::repo::object::{Blob, Commit, CommitInfo, Tree};
use chrono::{TimeZone, Utc};
use std::path::Path;
use std::str::FromStr;

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
    // commit format: `commit<NUL><commit>`
    let obj = format!("commit\0{}", commit).into_bytes();
    commit.update_hash(&obj);
    obj
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
pub fn deserialize_tree<P: AsRef<Path>>(path: P, obj: &[u8]) -> Option<Tree> {
    let mut tree = obj
        .strip_prefix(b"tree\0")
        .and_then(|o| deserialize_tree_entries(path, o))?;

    tree.update_hash(obj);
    Some(tree)
}

fn deserialize_tree_entries<P: AsRef<Path>>(path: P, obj: &[u8]) -> Option<Tree> {
    let mut tree = Tree::new();
    let mut it = obj.split(|&b| b == b'\0');
    let first = match it.next() {
        None => return Some(tree),
        Some(x) => x,
    };
    it.try_fold(first, |a, b| {
        let (kind, name) = (a.get(..4)?, a.get(5..)?);
        let (hash, next) = (b.get(..HASH_LENGTH)?, b.get(HASH_LENGTH..)?);
        let name = parse_string(name)?;
        let hash = parse_from_utf8(hash)?;
        match kind {
            b"blob" => tree.add_blob(hash, name, (*path.as_ref()).to_path_buf()),
            b"tree" => tree.add_tree(hash, name, (*path.as_ref()).to_path_buf()),
            _ => return None,
        };
        Some(next)
    })
    .and_then(|rest| if rest.is_empty() { Some(tree) } else { None })
}

/// Deserializes a commit object.
/// Returns None if obj is not a valid commit object.
pub fn deserialize_commit<P: AsRef<Path>>(path: P, obj: &[u8]) -> Option<Commit> {
    let mut commit = obj
        .strip_prefix(b"commit\0")
        .and_then(|o| deserialize_commit_data(path, o))?;

    commit.update_hash(obj);
    Some(commit)
}

fn deserialize_commit_data<P: AsRef<Path>>(path: P, obj: &[u8]) -> Option<Commit> {
    let mut it = obj.split(|&b| b == b'\n');
    let tree = parse_from_utf8(it.next()?.strip_prefix(b"tree ")?)?;
    let mut next = it.next()?;
    let parent = match next.strip_prefix(b"parent ") {
        None => None,
        Some(b) => {
            next = it.next()?;
            Some(parse_from_utf8(b)?)
        }
    };
    let author = parse_string(next.strip_prefix(b"author ")?)?;
    let time = parse_from_utf8(it.next()?.strip_prefix(b"time ")?)?;
    let time = Utc.timestamp_millis(time);
    it.next()?;
    let msg = parse_string(it.next()?)?;
    if !it.next()?.is_empty() {
        return None;
    }
    Some(Commit::new(CommitInfo {
        tree,
        parent,
        author,
        time,
        msg,
        path: (*(path.as_ref())).to_path_buf(),
    }))
}

fn parse_from_utf8<T: FromStr>(b: &[u8]) -> Option<T> {
    parse_string(b)?.parse().ok()
}

fn parse_string(b: &[u8]) -> Option<String> {
    String::from_utf8(b.to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::object::Hash;

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

    #[test]
    fn serde_commit() {
        let mut c1 = Commit::new(CommitInfo {
            tree: Hash::new(),
            parent: Some(Hash::new()),
            author: "paul".to_owned(),
            time: Utc.timestamp_millis(1637385703000),
            msg: "write some code".to_owned(),
        });

        let obj = serialize_commit(&mut c1);
        assert_eq!(obj, format!("commit\0{}", c1).into_bytes());

        let c2 = deserialize_commit(&obj).unwrap();
        assert_eq!(c1, c2);
    }

    #[test]
    fn serde_commit_no_parent() {
        let mut c1 = Commit::new(CommitInfo {
            tree: Hash::new(),
            parent: None,
            author: "paul".to_owned(),
            time: Utc.timestamp_millis(1637385703000),
            msg: "write some code".to_owned(),
        });

        let obj = serialize_commit(&mut c1);
        assert_eq!(obj, format!("commit\0{}", c1).into_bytes());

        let c2 = deserialize_commit(&obj).unwrap();
        assert_eq!(c1, c2);
    }
}
