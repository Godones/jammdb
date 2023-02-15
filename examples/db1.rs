use jammdb::memfile::{FakeMap, FileOpenOptions};
use jammdb::{Error, DB};
use std::sync::Arc;

fn main() -> Result<(), Error> {
    let path = std::path::Path::new("my-database.db");
    if path.exists() {
        std::fs::remove_file(path).unwrap();
    }
    let path = String::from("root-d");
    let old_name = String::from("old");
    let new_name = String::from("new");
    // open a new database file
    let db = DB::open::<FileOpenOptions, _>(Arc::new(FakeMap), "my-database.db")?;

    {
        let tx = db.tx(true)?;
        let r_bucket = tx.create_bucket(path.as_str())?;
        let data = r_bucket.create_bucket("data")?;
        data.put(old_name.to_string() + "-f", "")?;
        let old_file = tx.create_bucket("root/old-f")?;
        old_file.put("data", "")?;
        tx.commit()?;
    }

    {
        let tx = db.tx(true)?;
        let r_bucket = tx.get_bucket(path.as_str())?;
        let r_bucket = r_bucket.get_bucket("data")?;
        let old = r_bucket.get_kv(old_name.to_string() + "-f");
        let new = r_bucket.get_kv(new_name.to_string() + "-f");

        let _ans = if old.is_some() {
            if new.is_some() {
                Err(Error::InvalidDB("old file not found".to_string()))
            } else {
                let l = path.len();
                let old_path = path[0..l - 2].to_string() + "/" + &old_name + "-f";
                let new_path = path[0..l - 2].to_string() + "/" + &new_name + "-f";
                r_bucket.delete(old_name.to_string() + "-f")?;
                r_bucket.put(new_name.to_string() + "-f", "")?;
                println!("create new");

                let old_bucket = tx.get_bucket(old_path.clone())?;
                let old_data_p = old_bucket.get_kv("data").unwrap();
                let old_data = old_data_p.value().to_vec();
                tx.delete_bucket(old_path)?;
                let new_bucket = tx.create_bucket(new_path)?;
                new_bucket.put("data", old_data)?;
                tx.commit()?;
                Ok(())
            }
        } else {
            Err(Error::InvalidDB("old file not found".to_string()))
        };
    }

    println!("test jammdb ok");
    Ok(())
}
