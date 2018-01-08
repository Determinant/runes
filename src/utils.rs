use core::mem::{transmute, size_of};
use core::slice::{from_raw_parts_mut, from_raw_parts};

pub struct Sampler {
    freq2: u32,
    q0: u32,
    r0: u32,
    ddl: (u32, u32),
    cnt: u32,
    sec_cnt: u32
}

impl Sampler {
    pub fn new(freq1: u32, freq2: u32) -> Self {
        let q0 = freq1 / freq2;
        let r0 = freq1 - q0 * freq2;
        Sampler {
            freq2,
            q0,
            r0,
            ddl: (q0, r0),
            cnt: 0,
            sec_cnt: 0
        }
    }

    pub fn load(&mut self, reader: &mut Read) -> bool {
        load_prefix(self, 0, reader)
    }

    pub fn save(&self, writer: &mut Write) -> bool {
        save_prefix(self, 0, writer)
    }

    pub fn tick(&mut self) -> bool {
        let (q, r) = self.ddl;
        if self.cnt == q {
            let nr = r + self.r0;
            self.ddl = if nr > self.freq2 {
                (self.q0, nr - self.freq2)
            } else {
                (self.q0 - 1, nr)
            };
            self.cnt = 0;
            self.sec_cnt += 1;
            if self.sec_cnt == self.freq2 {
                self.sec_cnt = 0
            }
            true
        } else {
            self.cnt += 1;
            false
        }
    }
}

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize>;
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Option<usize>;
}

pub fn load_prefix<T>(obj: &mut T, ignored: usize, reader: &mut Read) -> bool {
    let len = size_of::<T>() - ignored;
    match reader.read(unsafe {
        from_raw_parts_mut(
            transmute::<*mut T, *mut u8>(obj as *mut T),
            len
    )}) {
        Some(x) => x == len,
        None => false
    }
}

pub fn save_prefix<T>(obj: &T, ignored: usize, writer: &mut Write) -> bool {
    let len = size_of::<T>() - ignored;
    match writer.write(unsafe {
        from_raw_parts(
            transmute::<*const T, *const u8>(obj as *const T),
            len
            )}) {
        Some(x) => x == len,
        None => false
    }
}
