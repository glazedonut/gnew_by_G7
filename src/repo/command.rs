use crate::repo::object::{Blob, Hash, Repository};
use crate::storage::transport::{self, Result};
use std::path::Path;

pub fn init() -> Result<()> {
    Repository::create_empty()?;
    Ok(())
}

pub fn add<P: AsRef<Path>>(files: &Vec<P>) -> Result<()> {
    Repository::add_to_tracklist(files)
}
pub fn commit(commmitmsg:Option<String>)->Result<()>{
    let mut cmsg:String="";
    let mut r = Repository::from_disc()?;

    todo!();
}
pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<Blob> {
    transport::write_blob(path)
}

pub fn cat_object(object: &Hash) -> Result<Vec<u8>> {
    transport::read_blob(object).map(|blob| blob.content().into())
}
