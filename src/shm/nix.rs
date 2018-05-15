use nix::libc::c_void;
use nix::Error as NixError;
use nix::fcntl::OFlag;
use nix::libc::off_t;
use nix::sys::mman::{mmap, munmap, shm_open, shm_unlink, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use nix::unistd::{close, ftruncate};

use rand::{self, Rng};

use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::ptr;

use error::*;

impl From<NixError> for Error {
    fn from(n: NixError) -> Self {
        Error::System(Box::new(n))
    }
}

#[derive(Debug)]
struct Shm {
    len: usize,
    raw_fd: RawFd,
}

impl Shm {
    pub fn create() -> Result<Self> {
        let mut rng = rand::thread_rng();
        let a: u64 = rng.gen();
        let b: u64 = rng.gen();

        let name = format!("{:x}{:x}", a, b);

        let mut name_buf = PathBuf::from("/");
        name_buf.push(name);

        let result = shm_open(
            &name_buf,
            OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_EXCL,
            Mode::empty(),
        )?;

        if let Err(e) = shm_unlink(&name_buf) {
            close(result)?;
            return Err(e.into());
        }

        Ok(Shm {
            len: 0,
            raw_fd: result,
        })
    }

    pub fn resize(&mut self, len: usize) -> Result<()> {
        if len > off_t::max_value() as usize {
            panic!("value for len too large");
        }

        ftruncate(self.raw_fd, len as off_t)?;

        self.len = len;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl AsRawFd for Shm {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_fd
    }
}

impl Drop for Shm {
    fn drop(&mut self) {
        close(self.raw_fd).ok();
    }
}

struct ShmMap {
    len: usize,
    data: *mut u8,
}

impl ShmMap {
    pub fn create(shm: &Shm) -> Result<Self> {
        let fd = shm.as_raw_fd();

        let mapped = unsafe {
            mmap(
                ptr::null_mut(),
                shm.len() as usize,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd,
                0,
            )?
        };

        Ok(ShmMap {
            len: shm.len(),
            data: mapped as *mut u8,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.data
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for ShmMap {
    fn drop(&mut self) {
        unsafe {
            munmap(self.data as *mut c_void, self.len).ok();
        }
    }
}

pub struct SharedMemory {
    shm: Shm,
    map: ShmMap,
}

impl SharedMemory {
    pub fn create<P: AsRef<Path>>(_name: P, capacity: usize) -> Result<Self> {
        let mut shm = Shm::create()?;
        shm.resize(capacity)?;

        let map = ShmMap::create(&shm)?;

        let writer = SharedMemory {
            map,
            shm,
        };

        Ok(writer)
    }

    pub fn open<P: AsRef<Path>>(_name: P) -> Result<Self> {
        unimplemented!()
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.map.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.map.as_mut_ptr()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}
