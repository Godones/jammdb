use jammdb::memfile::{FakeMap, FileOpenOptions};
use jammdb::{Error, DB};
use std::sync::Arc;

fn main() -> Result<(), Error> {
    let path = std::path::Path::new("my-database.db");
    if path.exists() {
        std::fs::remove_file(path).unwrap();
    }
    // open a new database file
    let db = DB::open::<FileOpenOptions, _>(Arc::new(FakeMap), "my-database.db")?;

    {
        let tx = db.tx(true)?;
        let bucket = tx.create_bucket("root")?;
        bucket.put("key", "value")?;
        tx.commit()?;
    }
    let tx = db.tx(true)?;
    {
        let bucket = tx.get_bucket("root")?;
        let value = bucket.get_kv("key").unwrap();
        let value = value.value();
        assert_eq!(value, "value".as_bytes());
        tx.delete_bucket("root")?;
        let bucket = tx.create_bucket("toot")?;
        bucket.put("key", "value")?;
    }
    tx.commit()?;
    Ok(())
}
