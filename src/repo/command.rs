use crate::repo::object::{Blob, Hash, Repository};
use crate::storage::transport::{self, Result};
use std::fs::metadata;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

pub fn init() -> Result<()> {
    Repository::create_empty()?;
    Ok(())
}

/* adds the specfied files to the tracklist on disc */
pub fn add<P: AsRef<Path>>(files: &Vec<P>) -> Result<()> {
    /* check that all the specified files exist */
    transport::check_existence(files)?;

    /* read current state of repository from disc */
    let mut r = Repository::from_disc()?;

    /* if file isn't tracked already, add it to tracklist
     * note that this adds directories just like files
     * during commit, dirs have to be added recursively
     */
    for f in files {
        let mut paths: Vec<String> = Vec::new();
        let md = metadata(f).unwrap();

        /* if a file, just add it to the path */
        if md.is_file() {
            paths.push(f.as_ref().to_str().unwrap().to_string());
        } else if md.is_dir() {
            /* if a dir, walk it adding files */
            /* Code inspired by an example in https://github.com/BurntSushi/walkdir/blob/master/README.md */
            let dir_walker = WalkDir::new(f).into_iter();
            /* only grab entries that do not start with a . */
            for entry in dir_walker.filter_entry(|e| !is_hidden(e)) {
                let entry = entry.unwrap();
                let path = entry.path();

                /* only add files, not directory names */
                let p_md = metadata(path).unwrap();
                if (p_md.is_file()) {
                    paths.push(path.to_str().unwrap().to_string());
                }
            }
        }

        /* add all valid paths to the tracklist */
        for path in paths {
            if !r.tracklist.contains(&path) {
                r.tracklist.push(path);
            }
        }
    }

    // TODO replace generic function with wrapper for tracklist
    /* write new tracklist to .gnew/tracklist */
    transport::write_lines_gen(Path::new(".gnew/tracklist"), &r.tracklist)?;

    Ok(())
}

pub fn commit(commitmsg: Option<String>) -> Result<()> {
    let mut cmsg: Option<String> = Some("".to_string());
    match commitmsg {
        Some(c) => cmsg = Some(c),
        None => cmsg = Some("".to_string()),
    };
    let mut r = Repository::from_disc()?;

    todo!();
}

pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<Blob> {
    transport::write_blob(path)
}

pub fn cat_object(object: &Hash) -> Result<Vec<u8>> {
    transport::read_blob(object).map(|blob| blob.content().into())
}

/* check that a direntry starts with a . */
fn is_hidden(entry: &DirEntry) -> bool {
    println!("{:?}", entry.path().to_str());
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
