use crate::repo::object::{Blob, Hash, Repository};
use crate::storage::transport::{self, Result, Error};
use std::path::Path;

pub fn init() -> Result<()> {
    match Repository::create_empty() {
        Ok(_) => Ok(()),
        Err(e) => Err(e)
    }
}

pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<Blob> {
    transport::write_blob(path)
}

pub fn cat_object(object: &Hash) -> Result<Vec<u8>> {
    transport::read_blob(object).map(|blob| blob.content().into())
}
