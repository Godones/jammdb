//! 文件接口定义
//!
//! jammdb依赖操作系统的文件系统接口，在no_std环境下无法直接使用，
//! 因此这里自定义了文件接口，以便在no_std环境下使用。
pub mod memfile;

use alloc::boxed::Box;
use alloc::string::ToString;
use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use core2::io::{Read, Seek, Write};


pub type IOResult<T> = core2::io::Result<T>;

pub struct File {
    pub file: Box<dyn DbFile>,
}

impl File {
    pub fn new(file: Box<dyn DbFile>) -> Self {
        Self { file }
    }
}
impl Deref for File {
    type Target = dyn DbFile;
    fn deref(&self) -> &Self::Target {
        self.file.as_ref()
    }
}

impl DerefMut for File {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.file.as_mut()
    }
}

/// include the file len
pub struct MetaData {
    pub len: u64,
}

impl MetaData {
    /// get the file len
    pub fn len(&self) -> u64 {
        self.len
    }
}

pub trait FileExt {
    fn lock_exclusive(&self) -> IOResult<()>;
    fn allocate(&mut self, new_size: u64) -> IOResult<()>;
    fn unlock(&self) -> IOResult<()>;
    fn metadata(&self) -> IOResult<MetaData>;
    fn sync_all(&self) -> IOResult<()>;
    fn size(&self) -> usize;
    fn addr(&self) -> usize;
}
/// fake trait
pub trait OpenOption {
    fn new() -> Self;
    fn read(&mut self, read: bool) -> &mut Self;
    fn write(&mut self, write: bool) -> &mut Self;
    fn open<T: ToString + PathLike>(&mut self, path: &T) -> IOResult<File>;
    fn create(&mut self, create: bool) -> &mut Self;
}

pub trait PathLike:Display+Debug {
    fn exists(&self) -> bool;
}

pub trait DbFile: Seek + Write + Read + FileExt {}

pub trait MemoryMap: Deref<Target = [u8]> {
    fn map(file: &mut dyn DbFile) -> IOResult<Self>
    where
        Self: Sized;
}
