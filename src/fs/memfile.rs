use crate::fs::{DbFile, File, FileExt, IOResult, MemoryMap, MetaData, OpenOption, PathLike};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Add;

use crate::{IndexByPageID, Mmap};
use core2::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref FILE_S: Mutex<HashMap<String, MemoryFile>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Clone)]
pub struct MemoryFile {
    pub name: String,
    pub pos: usize,
    pub data: Vec<u8>,
}

impl Seek for MemoryFile {
    /// seek
    fn seek(&mut self, pos: SeekFrom) -> IOResult<u64> {
        //info!("seek: {:?}", pos);
        match pos {
            SeekFrom::Start(l) => self.pos = l as usize,
            SeekFrom::Current(l) => self.pos += l as usize,
            SeekFrom::End(l) => {
                if l.unsigned_abs() as usize > self.data.len() {
                    return Err(core2::io::Error::new(ErrorKind::Other, "seek error"));
                } else {
                    self.pos += l as usize;
                }
            }
        };
        FILE_S.lock().get_mut(self.name.as_str()).unwrap().pos = self.pos;
        Ok(self.pos as u64)
    }
}

impl Read for MemoryFile {
    /// read
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        //info!("read buf len: {}", buf.len());
        let act_size = self.data.len().saturating_sub(self.pos);
        let act_size = if act_size > buf.len() {
            buf.len()
        } else {
            act_size
        };
        let addr = unsafe { self.data.as_ptr().add(self.pos) };
        unsafe {
            core::ptr::copy(addr, buf.as_mut_ptr(), act_size);
        }
        self.pos += act_size;
        FILE_S.lock().get_mut(self.name.as_str()).unwrap().pos = self.pos;
        Ok(act_size)
    }
}

impl Write for MemoryFile {
    /// write
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        //info!("write buf len: {}", buf.len());
        let act_size = buf.len() + self.pos;
        if act_size > self.data.len() {
            self.data.resize(act_size, 0);
        }
        self.data[self.pos..act_size].copy_from_slice(buf);
        self.pos += buf.len();
        FILE_S.lock().insert(self.name.clone(), self.clone());
        Ok(buf.len())
    }
    /// flush
    fn flush(&mut self) -> IOResult<()> {
        Ok(())
    }
}

#[allow(unused)]
/// 文件读取
pub struct FileOpenOptions;

impl OpenOption for FileOpenOptions {
    /// new
    fn new() -> Self {
        FileOpenOptions
    }
    /// set the read
    fn read(&mut self, _: bool) -> &mut Self {
        self
    }
    /// set the write
    fn write(&mut self, _: bool) -> &mut Self {
        self
    }
    /// open file
    fn open<T: ToString + PathLike>(&mut self, path: &T) -> IOResult<File> {
        let file = MemoryFile::open(path);

        match file {
            Some(f) => Ok(File::new(Box::new(f))),
            None => Err(core2::io::Error::new(ErrorKind::Other, "open file error")),
        }
    }
    /// create file
    fn create(&mut self, _: bool) -> &mut Self {
        self
    }
}

impl MemoryFile {
    /// create or get file
    pub fn open<T: PathLike + ToString>(name: &T) -> Option<Self> {
        //info!("open file {}", name);
        if FILE_S.lock().get(&name.to_string()).is_some() {
            let mut file = FILE_S.lock().get(&name.to_string()).unwrap().clone();
            file.pos = 0;
            FILE_S.lock().insert(name.to_string(), file.clone());
            return Some(file);
        }
        let file = Self {
            name: name.to_string(),
            pos: 0,
            data: Vec::new(),
        };
        FILE_S.lock().insert(name.to_string(), file.clone());
        Some(file)
    }
}

impl FileExt for MemoryFile {
    /// lock file
    fn lock_exclusive(&self) -> IOResult<()> {
        Ok(())
    }
    /// 扩展大小
    fn allocate(&mut self, new_size: u64) -> IOResult<()> {
        //info!(
        //     "before allocate: {:?}, new_size:{:#x}",
        //     self.data.len(),
        //     new_size
        // );
        if self.data.len() > new_size as usize {
            return Ok(());
        }
        self.data.resize(new_size as usize, 0);
        FILE_S.lock().insert(self.name.clone(), self.clone());
        //info!("after allocate: {:#x}", self.data.len());
        Ok(())
    }
    fn unlock(&self) -> IOResult<()> {
        Ok(())
    }

    /// get the metadata
    fn metadata(&self) -> IOResult<MetaData> {
        let data = MetaData {
            len: self.data.len() as u64,
        };
        Ok(data)
    }

    /// sync all
    fn sync_all(&self) -> IOResult<()> {
        Ok(())
    }

    fn size(&self) -> usize {
        self.data.len()
    }

    fn addr(&self) -> usize {
        self.data.as_ptr() as usize
    }
}

impl DbFile for MemoryFile {}

impl PathLike for &str {
    fn exists(&self) -> bool {
        FILE_S.lock().contains_key(self.to_string().as_str())
    }
}

impl PathLike for &String {
    fn exists(&self) -> bool {
        FILE_S.lock().contains_key(self.as_str())
    }
}

/// memory map
#[derive(Clone)]
pub struct FakeMap;

impl MemoryMap for FakeMap {
    /// map
    fn map(&self, file: &mut File) -> Result<Mmap, core2::io::Error> {
        //info!("[{}/{}] map file: {:#x}", file!(), line!(), file.size());
        let map = Mmap {
            size: file.size(),
            addr: file.addr(),
        };
        Ok(map)
    }
    fn do_map(&self, file: &mut File) -> IOResult<Arc<dyn IndexByPageID>> {
        let t = IndexByPageIDImpl {
            size: file.size(),
            addr: file.addr(),
        };
        Ok(Arc::new(t))
    }
}

struct IndexByPageIDImpl {
    size: usize,
    addr: usize,
}

impl IndexByPageID for IndexByPageIDImpl {
    fn index(&self, page_id: u64, page_size: usize) -> IOResult<&[u8]> {
        if (page_size * page_id as usize) > self.size {
            panic!("index is out of range");
        }
        let addr = self.addr.add(page_id as usize * page_size);
        let data = unsafe { core::slice::from_raw_parts(addr as *const u8, page_size) };
        Ok(data)
    }
}
