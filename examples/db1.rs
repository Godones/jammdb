use std::collections::HashMap;
use jammdb::{Data, Error, DB, OpenOptions};
use jammdb::memfile::{FileOpenOptions, Mmap};

fn main() -> Result<(), Error> {
    let path = std::path::Path::new("my-database.db");
    if path.exists(){
        std::fs::remove_file(path).unwrap();
    }
    {
        // open a new database file
        let db = DB::<Mmap>::open::<FileOpenOptions, _>("my-database.db")?;

        // open a writable transaction so we can make changes
        let tx = db.tx(true)?;

        // create a bucket to store a map of first names to last names
        let mut names_bucket = tx.create_bucket("names")?;
        for i in 0..10 {
            names_bucket = names_bucket.create_bucket(format!("names{}", i))?;
        }

        names_bucket.put("Kanan", "Jarrus")?;
        names_bucket.put("Ezra", "Bridger")?;

        // commit the changes so they are saved to disk
        tx.commit()?;
    }
    {
        // open the existing database file
        let db = DB::<Mmap>::open::<FileOpenOptions,_>("my-database.db")?;
        // open a read-only transaction to get the data
        let tx = db.tx(true)?;
        // get the bucket we created in the last transaction
        let names_bucket = tx.get_bucket("names")?;
        // get the key / value pair we inserted into the bucket
        if let Some(data) = names_bucket.get("Kanan") {
            assert_eq!(data.kv().value(), b"Jarrus");
        }
    }
    println!("test jammdb ok");
    Ok(())
}
