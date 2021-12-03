use crate::repo::object::Repository;
use crate::storage::transport::{self, Result};
use std::fs::metadata;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

// TODO: change return to vec of strings for printing. For now, we just print here
pub fn heads() -> Result<()> {
    let r = Repository::open()?;
    let heads = r.branches();

    for h in heads {
        println!("{:?}", h);
    }

    Ok(())
}

/* check that a direntry starts with a . */
fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
