use crate::fs::{DbFile, File, FileExt, IOResult, MemoryMap, MetaData, OpenOption, PathLike};
use alloc::alloc::{alloc, dealloc, Layout};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use core::ops::Deref;
use core2::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    /// 保存已经打开的文件
    pub static ref  FILE_S:Mutex<HashMap<String,MemoryFile>> = Mutex::new( HashMap::new());
}

#[derive(Debug, Clone)]
pub struct MemoryFile {
    pub name: String,
    pub pos: isize,
    pub size: usize,
    pub addr: usize,
}

impl Seek for MemoryFile {
    /// seek
    fn seek(&mut self, pos: SeekFrom) -> IOResult<u64> {
        info!("seek: {:?}", pos);
        match pos {
            SeekFrom::Start(l) => {
                self.pos = l as isize;
            }
            SeekFrom::Current(l) => {
                self.pos += l as isize;
            }
            SeekFrom::End(l) => {
                if l.abs() as usize > self.size {
                    return Err(core2::io::Error::new(ErrorKind::Other, "seek error"));
                } else {
                    self.pos += l as isize;
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
        let addr = self.addr;
        let addr = addr + self.pos as usize;
        let addr = addr as *const u8;
        let act_size = self.size.saturating_sub(self.pos as usize);
        unsafe {
            core::ptr::copy(addr, buf.as_mut_ptr(), act_size);
        }
        self.pos += act_size as isize;
        FILE_S.lock().get_mut(self.name.as_str()).unwrap().pos = self.pos;
        Ok(act_size)
    }
}

impl Write for MemoryFile {
    /// write
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        info!("write buf len: {}", buf.len());
        let addr = self.addr;
        let w_addr;
        let old_size = self.size;
        if self.size < self.pos as usize + buf.len() {
            // remalloc;
            self.size = self.pos as usize + buf.len();
            let layout = Layout::from_size_align(self.size, 8).unwrap();
            let addr = unsafe { alloc(layout) };
            let mut lock = FILE_S.lock();
            let file = lock.get_mut(self.name.as_str()).unwrap();
            let old_addr = file.addr;
            // 更新新地址
            file.addr = addr as usize;
            file.size = self.size;

            self.addr = addr as usize;
            //copy data
            unsafe {
                let old_addr = old_addr as *const u8;
                let addr = addr as *mut u8;
                core::ptr::copy(old_addr, addr, old_size);
                dealloc(
                    old_addr as *mut u8,
                    Layout::from_size_align(old_size, 8).unwrap(),
                );
            }
            w_addr = addr as usize + self.pos as usize;
        } else {
            w_addr = addr + self.pos as usize;
        }
        unsafe {
            core::ptr::copy(buf.as_ptr(), w_addr as *mut u8, buf.len());
        }
        self.pos += buf.len() as isize;
        FILE_S.lock().get_mut(self.name.as_str()).unwrap().pos = self.pos;
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
        let ans = match file {
            Some(f) => Ok(File::new(Box::new(f))),
            None => Err(core2::io::Error::new(ErrorKind::Other, "open file error")),
        };
        ans
    }
    /// create file
    fn create(&mut self, _: bool) -> &mut Self {
        self
    }
}

impl MemoryFile {
    /// create or get file
    pub fn open<T: PathLike + ToString>(name: &T) -> Option<Self> {
        if FILE_S.lock().get(&name.to_string()).is_some() {
            let mut file = FILE_S.lock().get(&name.to_string()).unwrap().clone();
            file.pos = 0;
            FILE_S.lock().insert(name.to_string().clone(), file.clone());
            return Some(file);
        }
        let addr = unsafe { alloc(Layout::from_size_align(0, 8).unwrap()) };
        let file = Self {
            name: name.to_string().clone(),
            pos: 0,
            size: 0,
            addr: addr as usize,
        };
        FILE_S.lock().insert(name.to_string().clone(), file.clone());
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
        info!("before allocate: {:?}, new_size:{:#x}", self.size, new_size);
        if self.size > new_size as usize {
            return Ok(());
        }
        let layout = Layout::from_size_align(new_size as usize, 8).unwrap();
        let addr = unsafe { alloc(layout) };
        let mut lock = FILE_S.lock();
        let file = lock.get_mut(self.name.as_str()).unwrap();
        let old_addr = file.addr;
        file.addr = addr as usize;
        self.addr = addr as usize;

        unsafe {
            let old_addr = old_addr as *const u8;
            let addr = addr as *mut u8;
            core::ptr::copy(old_addr, addr, self.size);
            dealloc(
                old_addr as *mut u8,
                Layout::from_size_align(self.size, 8).unwrap(),
            );
        }
        file.size = new_size as usize;
        self.size = new_size as usize;
        info!("after allocate: {:x?}", self.size);
        Ok(())
    }
    fn unlock(&self) -> IOResult<()> {
        Ok(())
    }

    /// get the metadata
    fn metadata(&self) -> IOResult<MetaData> {
        let data = MetaData {
            len: self.size as u64,
        };
        Ok(data)
    }

    /// sync all
    fn sync_all(&self) -> IOResult<()> {
        Ok(())
    }

    fn size(&self) -> usize {
        self.size
    }

    fn addr(&self) -> usize {
        self.addr
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
pub struct Mmap {
    size: usize,
    addr: usize,
}

impl MemoryMap for Mmap {
    /// map
    fn map(file: &mut dyn DbFile) -> Result<Self, core2::io::Error> {
        info!("[{}/{}] map file: {:x?}", file!(), line!(), file.size());
        let map = Mmap {
            size: file.size(),
            addr: file.addr(),
        };
        Ok(map)
    }
}

impl Deref for Mmap {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.addr as *const u8, self.size) }
    }
}
