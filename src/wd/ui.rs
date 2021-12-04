use crate::repo::object::{Change, FileStatus, Status};
use crate::storage::transport::Result;
use similar::TextDiff;
use std::io;
use std::path::{Path, PathBuf};

pub fn print_status(status: &Status) {
    for (path, fstatus) in status {
        match fstatus {
            FileStatus::Unmodified => (),
            _ => println!("{} {}", fstatus.code(), path.display()),
        }
    }
}

/// Outputs the changes as a unified diff.
pub fn print_diff(changes: &[Change]) -> Result<()> {
    changes.iter().try_for_each(print_file_diff)
}

fn print_file_diff(change: &Change) -> Result<()> {
    let (old, new) = change.contents()?;
    let (a, b) = diff_header(change);

    Ok(TextDiff::from_lines(&old, &new)
        .unified_diff()
        .header(&a.to_string_lossy(), &b.to_string_lossy())
        .to_writer(io::stdout())?)
}

fn diff_header(change: &Change) -> (PathBuf, PathBuf) {
    let (a, b) = match change {
        Change::Add(_) => (None, Some("b")),
        Change::Remove(_) => (Some("a"), None),
        Change::Modify(..) => (Some("a"), Some("b")),
    };
    let header_path = |f| match f {
        None => PathBuf::from("/dev/null"),
        Some(p) => Path::new(p).join(change.path()),
    };
    (header_path(a), header_path(b))
}
