#![allow(dead_code)]

use alloc::ffi::CString;
use core::ffi::c_int;
use core::ffi::c_void;

use no_std_io2::io;
use no_std_io2::io::{Read, Seek, SeekFrom, Write};
use playdate::{println, sys as playdate_sys};
use playdate::sys::ffi::{FileOptions, SDFile};

pub struct FileHandle {
    handle: *mut SDFile,
}

// SAFETY: Playdate is single-threaded
unsafe impl Send for FileHandle {}
unsafe impl Sync for FileHandle {}

impl FileHandle {
    /// Opens a handle for the file at path.
    ///
    /// The kFileRead mode opens a file in the game pdx, while kFileReadData searches
    /// the game’s data folder; to search the data folder first then fall back on the game pdx,
    /// use the bitwise combination kFileRead|kFileReadData.
    /// kFileWrite and kFileAppend always write to the data folder.
    ///
    /// The function returns Err if the path contains a \0 byte (see [`CString::new`]).
    /// The function returns Err if the file at path cannot be opened, and will log the error to the console.
    /// The filesystem has a limit of 64 simultaneous open files.
    pub fn open(path: &str, mode: FileOptions) -> io::Result<Self> {
        let c_path = CString::new(path)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?;
        let handle = unsafe { playdate_sys::api!(file).open.unwrap()(c_path.as_ptr(), mode) };
        if handle.is_null() {
            let message = unsafe { playdate_sys::api!(file).geterr.unwrap()() };
            unsafe { playdate_sys::api!(system).logToConsole.unwrap()(message) };

            Err(io::Error::new(io::ErrorKind::Other, "Failed to open file"))
        } else {
            Ok(FileHandle { handle })
        }
    }

    /// Opens a handle for a file at path.
    /// Shorthand for [`Self::open`] with kFileRead and kFileReadData
    #[inline]
    pub fn read_only(path: &str) -> io::Result<Self> {
        Self::open(path, FileOptions::kFileRead | FileOptions::kFileReadData)
    }

    /// Opens a handle for a file at path.
    /// Shorthand for [`Self::open`] with kFileWrite (and kFileAppend if append = true)
    #[inline]
    pub fn write_only(path: &str, append: bool) -> io::Result<Self> {
        let append = if append {
            FileOptions::kFileAppend
        } else {
            FileOptions(0)
        };
        Self::open(path, FileOptions::kFileWrite | append)
    }

    /// Opens a handle for a file at path.
    /// Shorthand for [`Self::open`] with kFileRead, kFileReadData, kFileWrite (and kFileAppend if append = true)
    #[inline]
    pub fn read_write(path: &str, append: bool) -> io::Result<Self> {
        let append = if append {
            FileOptions::kFileAppend
        } else {
            FileOptions(0)
        };
        Self::open(
            path,
            FileOptions::kFileRead | FileOptions::kFileReadData | FileOptions::kFileWrite | append,
        )
    }
}

impl Read for FileHandle {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = unsafe {
            playdate_sys::api!(file).read.unwrap()(
                self.handle,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,
            )
        };
        // println!("read: {}", result);
        if result < 0 {
            Err(io::Error::new(io::ErrorKind::Other, "Read error"))
        } else {
            Ok(result as usize)
        }
    }
}

impl Write for FileHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = unsafe {
            playdate_sys::api!(file).write.unwrap()(
                self.handle,
                buf.as_ptr() as *const c_void,
                buf.len() as u32,
            )
        };
        if result < 0 {
            Err(io::Error::new(io::ErrorKind::Other, "Write error"))
        } else {
            Ok(result as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = unsafe { playdate_sys::api!(file).flush.unwrap()(self.handle) };
        if result < 0 {
            Err(io::Error::new(io::ErrorKind::Other, "Flush error"))
        } else {
            Ok(())
        }
    }
}

impl Seek for FileHandle {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let (offset, whence) = match pos {
            SeekFrom::Start(n) => (n as c_int, 0),
            SeekFrom::End(n) => (n as c_int, 2),
            SeekFrom::Current(n) => (n as c_int, 1),
        };
        let result = unsafe { playdate_sys::api!(file).seek.unwrap()(self.handle, offset, whence) };
        if result < 0 {
            Err(io::Error::new(io::ErrorKind::Other, "Seek error"))
        } else {
            Ok(result as u64)
        }
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        unsafe { playdate_sys::api!(file).close.unwrap()(self.handle) };
    }
}

pub struct BufferedWriter<W: Write, const N: usize> {
    inner: W,
    buffer: [u8; N],
    pos: usize,
}

impl<W: Write> BufferedWriter<W, 1024> {
    pub fn new_default(inner: W) -> Self {
        Self::new(inner)
    }
}

impl<W: Write, const N: usize> BufferedWriter<W, N> {    
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: [0; N],
            pos: 0,
        }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.pos > 0 {
            self.inner.write_all(&self.buffer[..self.pos])?;
            self.pos = 0;
        }
        self.inner.flush()
    }
}

impl<W: Write, const N: usize> Write for BufferedWriter<W, N> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() >= N {
            self.flush()?;
            return self.inner.write(buf);
        }

        if self.pos + buf.len() > N {
            self.flush()?;
        }

        self.buffer[self.pos..self.pos + buf.len()].copy_from_slice(buf);
        self.pos += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        BufferedWriter::flush(self)
    }
}

impl<W: Write, const N: usize> Drop for BufferedWriter<W, N> {
    fn drop(&mut self) {
        let _ = self.flush(); // Ignore errors in Drop
    }
}

pub struct BufferedReader<R: Read, const N: usize> {
    inner: R,
    buffer: [u8; N],
    pos: usize,
    end: usize,
}

impl<R: Read, const N: usize> BufferedReader<R, N> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: [0; N],
            pos: 0,
            end: 0,
        }
    }

    pub fn fill_buffer(&mut self) -> io::Result<()> {
        self.pos = 0;
        self.end = self.inner.read(&mut self.buffer)?;
        Ok(())
    }
}

impl<R: Read, const N: usize> Read for BufferedReader<R, N> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos == self.end {
            self.fill_buffer()?;
            if self.end == 0 {
                return Ok(0);
            }
        }

        let len = buf.len().min(self.end - self.pos);
        buf[..len].copy_from_slice(&self.buffer[self.pos..self.pos + len]);
        self.pos += len;
        Ok(len)
    }
}
