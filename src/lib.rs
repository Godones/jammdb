//! # Just Another Memory Mapped Database
//!
//! jammdb is an embedded, single-file database that allows you to store key / value pairs as bytes.
//!
//! It started life as a Rust port of [Ben Johnson's](https://twitter.com/benbjohnson) [BoltDB](https://github.com/boltdb/bolt),
//! which was inspired by [Howard Chu's](https://twitter.com/hyc_symas) [LMDB](http://symas.com/mdb/),
//! so please check out both of these awesome projects!
//!
//! jammdb offers
//! [ACID](https://en.wikipedia.org/wiki/ACID) compliance,
//! [serializable](https://en.wikipedia.org/wiki/Serializability) and
//! [isolated](https://en.wikipedia.org/wiki/Isolation_(database_systems)) transactions,
//! with multiple lock-free readers and a single concurrent writer. The data is organized in a
//! [single level](https://en.wikipedia.org/wiki/Single-level_store) [B+ tree](https://en.wikipedia.org/wiki/B%2B_tree)
//! so random and sequential reads are very fast. The underlying file is [memory mapped](https://en.wikipedia.org/wiki/Memory-mapped_file), so reads require no additional memory allocation.
//!
//! jammdb is meant to be very simple, and has only a few exported types. It will allow you to store data in collections (called [`Buckets`](struct.Bucket.html)),
//! and each bucket can contain any number of unique keys which map to either an arbitrary value (a `&[u8]`) or a nested bucket. Examples on how to use jammdb are below.
//! There are also more examples in the docs, be sure to check out
//! * Using a [`Cursor`] to iterate over the data in a bucket
//! * How to create and use multiple [`Tx`]s
//! * Nested [`Buckets`](struct.Bucket.html)
//! * [`OpenOptions`](struct.OpenOptions.html) to provide parameters for opening a [`DB`](struct.DB.html)
//!
//! # Examples
//!
//! ## Simple put and get
//! ```no_run
//! use jammdb::{DB, Data, Error};
//!
//! fn main() -> Result<(), Error> {
//! use jammdb::memfile::{FileOpenOptions, Mmap};
//! {
//!     // open a new database file
//!     let db =  DB::<Mmap>::open::<FileOpenOptions,_>("my-database.db")?;
//!
//!     // open a writable transaction so we can make changes
//!     let mut tx = db.tx(true)?;
//!
//!     // create a bucket to store a map of first names to last names
//!     let mut names_bucket = tx.create_bucket("names")?;
//!     names_bucket.put("Kanan", "Jarrus")?;
//!     names_bucket.put("Ezra", "Bridger")?;
//!
//!     // commit the changes so they are saved to disk
//!     tx.commit()?;
//! }
//! {
//!     // open the existing database file
//!     let db =  DB::<Mmap>::open::<FileOpenOptions,_>("my-database.db")?;
//!     // open a read-only transaction to get the data
//!     let mut tx = db.tx(true)?;
//!     // get the bucket we created in the last transaction
//!     let names_bucket = tx.get_bucket("names")?;
//!     // get the key / value pair we inserted into the bucket
//!     if let Some(data) = names_bucket.get("Kanan") {
//!         assert_eq!(data.kv().value(), b"Jarrus");
//!     }
//! }
//!     Ok(())
//! }
//! ```
//!
//! ## Storing structs
//! ```no_run
//! use jammdb::{DB, Data, Error};
//! use serde::{Deserialize, Serialize};
//! // use rmps crate to serialize structs using the MessagePack format
//! use rmp_serde::{Deserializer, Serializer};
//!
//! #[derive(Debug, PartialEq, Deserialize, Serialize)]
//! struct User {
//!     username: String,
//!     password: String,
//! }
//!
//! fn main() -> Result<(), Error> {
//!     use jammdb::memfile::{FileOpenOptions, Mmap};let user = User{
//!         username: "my-user".to_string(),
//!         password: "my-password".to_string(),
//!     };
//! {
//!     // open a new database file and start a writable transaction
//!     let db = DB::<Mmap>::open::<FileOpenOptions,_>("my-database.db")?;
//!     let mut tx = db.tx(true)?;
//!
//!     // create a bucket to store users
//!     let mut users_bucket = tx.create_bucket("users")?;
//!
//!     // serialize struct to bytes and store in bucket
//!     let user_bytes = rmp_serde::to_vec(&user).unwrap();
//!     users_bucket.put("user1", user_bytes)?;
//!
//!     // commit the changes so they are saved to disk
//!     tx.commit()?;
//! }
//! {
//!     // open the existing database file
//!     let db =  DB::<Mmap>::open::<FileOpenOptions,_>("my-database.db")?;
//!     // open a read-only transaction to get the data
//!     let mut tx = db.tx(true)?;
//!     // get the bucket we created in the last transaction
//!     let users_bucket = tx.get_bucket("users")?;
//!     // get the key / value pair we inserted into the bucket
//!     if let Some(data) = users_bucket.get(b"user1") {
//!         // deserialize into a user struct
//!         let db_user: User = rmp_serde::from_slice(data.kv().value()).unwrap();
//!         assert_eq!(db_user, user);
//!     }
//! }
//!     Ok(())
//! }//
#![feature(error_in_core)]
#![no_std]
#[allow(clippy::mutable_key_type)]
mod bucket;
mod bytes;
mod cursor;
mod data;
mod db;
mod errors;
mod freelist;
mod fs;
mod lifetimes;
mod meta;
mod node;
mod page;
mod page_node;
mod tx;
extern crate alloc;
#[cfg(test)]
extern crate std;

pub use crate::bytes::ToBytes;
pub use bucket::Bucket;
pub use cursor::{Buckets, Cursor, KVPairs, ToBuckets, ToKVPairs};
pub use data::*;
pub use db::{OpenOptions, DB};
pub use errors::*;
pub use fs::memfile;
pub use fs::*;
pub use node::test_split;
pub use tx::Tx;

#[cfg(test)]
mod testutil {
    use crate::fs::PathLike;
    use crate::std::io::Write;
    use bytes::{BufMut, Bytes, BytesMut};
    use core::fmt::{Display, Formatter};
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::string::String;
    use std::vec::Vec;
    #[derive(Debug)]
    pub struct RandomFile {
        pub path: String,
    }

    impl Display for RandomFile {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "{}", self.path)
        }
    }

    impl RandomFile {
        pub fn new() -> RandomFile {
            loop {
                let filename: String = std::str::from_utf8(
                    rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(30)
                        .collect::<Vec<u8>>()
                        .as_slice(),
                )
                .unwrap()
                .into();
                let path = std::env::temp_dir().join(filename.clone());
                if path.metadata().is_err() {
                    return RandomFile { path: filename };
                }
            }
        }
    }

    impl PathLike for RandomFile {
        fn exists(&self) -> bool {
            let x = &self.path;
            x.exists()
        }
    }

    impl PathLike for &RandomFile {
        fn exists(&self) -> bool {
            let x = &self.path;
            x.exists()
        }
    }

    impl Drop for RandomFile {
        #[allow(unused_must_use)]
        fn drop(&mut self) {}
    }
    pub fn rand_bytes(size: usize) -> Bytes {
        let buf = BytesMut::new();
        let mut w = buf.writer();
        for byte in rand::thread_rng().sample_iter(&Alphanumeric).take(size) {
            let _ = write!(&mut w, "{}", byte);
            // let _ = w.write(&[byte]);
        }
        w.into_inner().freeze()
    }
}
