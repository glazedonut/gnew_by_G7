use crate::repo::object::Repository;
use crate::storage::transport::Result;

// TODO: change return to vec of strings for printing. For now, we just print here
pub fn heads() -> Result<()> {
    let r = Repository::open()?;
    let heads = r.branches();

    for h in heads {
        println!("{:?}", h);
    }

    Ok(())
}
