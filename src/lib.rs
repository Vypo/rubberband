#[cfg(unix)]
extern crate nix;
extern crate rand;

pub mod error;
mod shm;

use error::{Error, Result};

use shm::SharedMemory;

use std::mem;
use std::ops::{Index, IndexMut};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RubberBand {
    shm: SharedMemory,
    header: *const Header,
    data: *mut u8,
}

impl IndexMut<usize> for RubberBand {
    fn index_mut(&mut self, idx: usize) -> &mut u8 {
        let idx = idx % self.capacity();

        unsafe {
            &mut *self.data.offset(idx as isize)
        }
    }
}

impl Index<usize> for RubberBand {
    type Output = u8;

    fn index(&self, idx: usize) -> &u8 {
        let idx = idx % self.capacity();

        unsafe {
            &*self.data.offset(idx as isize)
        }
    }
}

impl RubberBand {
    const N_LEN: usize = mem::size_of::<usize>();

    fn capacity(&self) -> usize {
        self.shm.len() - mem::size_of::<Header>()
    }

    fn new(mut shm: SharedMemory) -> Result<Self> {
        let header = shm.as_mut_ptr() as *const Header;

        let band = RubberBand {
            shm,
            header,
            data: unsafe { header.offset(1) } as *mut u8,
        };

        Ok(band)
    }

    pub fn create<P: AsRef<Path>>(name: P, capacity: usize) -> Result<Self> {
        if capacity > isize::max_value() as usize {
            return Err(Error::TooBig);
        }

        let shm = SharedMemory::create(name, capacity + mem::size_of::<Header>())?;
        Self::new(shm)
    }

    pub fn open<P: AsRef<Path>>(name: P) -> Result<Self> {
        let shm = SharedMemory::open(name)?;
        Self::new(shm)
    }

    fn header(&self) -> &Header {
        unsafe {
            &*self.header
        }
    }
}

#[repr(C)]
struct Header {
    head: AtomicUsize,
    tail: AtomicUsize,
}

pub struct Sender {
    rubber_band: RubberBand,
}

impl Sender {
    fn header(&self) -> &Header {
        self.rubber_band.header()
    }

    pub fn create<P: AsRef<Path>>(name: P, capacity: usize) -> Result<Self> {
        let result = Self {
            rubber_band: RubberBand::create(name, capacity)?,
        };

        Ok(result)
    }

    pub fn open<P: AsRef<Path>>(name: P) -> Result<Self> {
        let result = Self {
            rubber_band: RubberBand::open(name)?,
        };

        Ok(result)
    }

    pub fn reserve<'a>(&'a mut self, size: usize) -> Result<Reservation<'a>> {
        let total_size = size + RubberBand::N_LEN;

        if total_size > self.rubber_band.capacity() {
            return Err(Error::TooBig);
        }

        let (head, tail) = {
            (
                self.header().head.load(Ordering::SeqCst),
                self.header().tail.load(Ordering::SeqCst),
            )
        };

        let used = if head > tail {
            head - tail
        } else {
            tail - head
        };

        let available = self.rubber_band.capacity() - used;

        if total_size > available {
            return Err(Error::Full);
        }

        let reserved = Reservation {
            sender: self,
            start: tail + RubberBand::N_LEN,
            len: size,
        };

        Ok(reserved)
    }

    fn commit(&mut self, len: usize) {
        let tail = self.header().tail.load(Ordering::SeqCst);

        let len_bytes: [u8; RubberBand::N_LEN] = unsafe { mem::transmute(len) };

        for (o, b) in len_bytes.iter().enumerate() {
            self.rubber_band[tail + o] = *b;
        }

        // TODO: Think about overflowing usize vs capacity
        let new_tail = tail + RubberBand::N_LEN + len;
        let new_tail = new_tail % self.rubber_band.capacity();

        self.header().tail.store(new_tail, Ordering::SeqCst);
    }
}

pub struct Reservation<'a> {
    sender: &'a mut Sender,
    start: usize,
    len: usize,
}

impl<'a> Index<usize> for Reservation<'a> {
    type Output = u8;

    fn index(&self, 
}

impl<'a> Reservation<'a> {
    pub fn commit(self) {
        self.sender.commit(self.len)
    }

    pub fn shrink(&mut self, len: usize) {
        if len > self.len {
            panic!("new length must be less than old length");
        }

        self.len = len;
    }
}
