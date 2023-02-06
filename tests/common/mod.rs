#[allow(dead_code)]
#[allow(clippy::mutable_key_type)]
pub mod record;

use core::fmt::{Display, Formatter};
use jammdb::PathLike;
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
pub fn rand_bytes(len: usize) -> Vec<u8> {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .collect::<Vec<u8>>()
}
