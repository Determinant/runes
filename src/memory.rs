#![allow(dead_code)]
use ppu::PPU;
use mos6502::CPU;
use cartridge::{MirrorType, Cartridge};
use controller::Controller;
use core::cell::{UnsafeCell, Cell};
use core::ptr::null_mut;

pub trait VMem {
    fn read(&self, addr: u16) -> u8;
    fn write(&self, addr: u16, data: u8);
}

pub struct CPUMemory<'a> {
    sram: UnsafeCell<[u8; 2048]>,
    ppu: Cell<*mut PPU<'a>>,
    cpu: Cell<*mut CPU<'a>>,
    mapper: &'a VMem,
    ctl1: Option<&'a Controller>,
    ctl2: Option<&'a Controller>
}

impl<'a> CPUMemory<'a> {
    pub fn new(ppu: *mut PPU<'a>,
               mapper: &'a VMem,
               ctl1: Option<&'a Controller>,
               ctl2: Option<&'a Controller>) -> Self {
        CPUMemory{sram: UnsafeCell::new([0; 2048]),
                  cpu: Cell::new(null_mut()),
                  ppu: Cell::new(ppu),
                  mapper, ctl1, ctl2}
    }

    pub fn init(&self, cpu: *mut CPU<'a>) {
        self.cpu.set(cpu);
    }
}

impl<'a> VMem for CPUMemory<'a> {
    fn read(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            unsafe{(*self.sram.get())[(addr & 0x07ff) as usize]}
        } else if addr < 0x4000 {
            let ppu = unsafe {&mut *self.ppu.get()};
            match addr & 0x7 {
                0x2 => ppu.read_status(),
                0x4 => ppu.read_oamdata(),
                0x7 => ppu.read_data(),
                _ => 0
            }
        } else if addr < 0x4020 {
            match addr {
                0x4016 => match self.ctl1 {
                    Some(c) => c.read(),
                    None => 0
                },
                0x4017 => match self.ctl2 {
                    Some(c) => c.read(),
                    None => 0
                },
                _ => 0
            }
        } else if addr < 0x6000 {
            0
        } else {
            self.mapper.read(addr)
        }
    }

    fn write(&self, addr: u16, data: u8) {
        let ppu = unsafe {&mut *self.ppu.get()};
        let cpu = unsafe {&mut *self.cpu.get()};
        if addr < 0x2000 {
            unsafe{(*self.sram.get())[(addr & 0x07ff) as usize] = data;}
        } else if addr < 0x4000 {
            match addr & 0x7 {
                0x0 => ppu.write_ctl(data),
                0x1 => ppu.write_mask(data),
                0x3 => ppu.write_oamaddr(data),
                0x4 => ppu.write_oamdata(data),
                0x5 => ppu.write_scroll(data),
                0x6 => ppu.write_addr(data),
                0x7 => ppu.write_data(data),
                _ => panic!("invalid ppu reg write access at 0x{:04x}", addr)
            }
        } else if addr < 0x4020 {
            match addr {
                0x4014 => ppu.write_oamdma(data, cpu),
                0x4016 => match self.ctl1 {
                    Some(c) => c.write(data),
                    None => ()
                },
                _ => ()
            }
        } else if addr < 0x6000 {
        } else {
            self.mapper.write(addr, data)
        }
    }
}

pub struct PPUMemory<'a> {
    nametable: UnsafeCell<[u8; 0x800]>,
    palette: UnsafeCell<[u8; 0x20]>,
    cart: &'a Cartridge,
    mapper: &'a VMem,
}

impl<'a> PPUMemory<'a> {
    pub fn new(mapper: &'a VMem,
               cart: &'a Cartridge) -> Self {
        PPUMemory{
            nametable: UnsafeCell::new([0; 0x800]),
            palette: UnsafeCell::new([0; 0x20]),
            cart,
            mapper}
    }

    pub fn dump(&self) {
        unsafe {
            for (i, v) in (*self.palette.get()).iter().enumerate() {
                print!("{:02x} ", *v);
                if (i & 0x7) == 0x7 {println!("@{:02x}", i)}
            }
            for (i, v) in (*self.nametable.get()).iter().enumerate() {
                print!("{:02x} ", *v);
                if (i & 0x1f) == 0x1f {println!("@{:02x}", i)}
            }
        }
    }
}

const MIRROR_IDX: [[u8; 4]; 5] = [
    [0, 0, 1, 1],
    [0, 1, 0, 1],
    [0, 0, 0, 0],
    [1, 1, 1, 1],
    [0, 1, 2, 3],
];

#[inline(always)]
fn get_mirror_addr(kind: MirrorType, addr: u16) -> u16 {
    let table = addr >> 10;
    let offset = addr & 0x03ff;
    ((MIRROR_IDX[kind as usize][table as usize] as u16) << 10) + offset
}

#[inline(always)]
fn get_mirror_palette(addr: u16) -> u16 {
    if addr >= 0x10 && addr & 3 == 0 {
        addr - 0x10
    } else { addr }
}

impl<'a> PPUMemory<'a> {
    #[inline(always)]
    pub fn read_nametable(&self, addr: u16) -> u8 {
        let kind = self.cart.mirror_type;
        unsafe {(*self.nametable.get())[(get_mirror_addr(kind, addr) & 0x7ff) as usize]}
    }

    #[inline(always)]
    pub fn read_palette(&self, addr: u16) -> u8 { unsafe {
        (*self.palette.get())[get_mirror_palette(addr) as usize]
    }}

    #[inline(always)]
    pub fn write_nametable(&self, addr: u16, data: u8) {
        let kind = self.cart.mirror_type;
        unsafe {(*self.nametable.get())[(get_mirror_addr(kind, addr) & 0x7ff) as usize] = data}
    }

    #[inline(always)]
    pub fn write_palette(&self, addr: u16, data: u8) { unsafe {
        (*self.palette.get())[get_mirror_palette(addr) as usize] = data;
    }}

    #[inline(always)]
    pub fn read_mapper(&self, addr: u16) -> u8 {
        self.mapper.read(addr)
    }

    #[inline(always)]
    fn write_mapper(&self, addr: u16, data: u8) {
        self.mapper.write(addr, data);
    }
}

impl<'a> VMem for PPUMemory<'a> {
    fn read(&self, mut addr: u16) -> u8 {
        addr &= 0x3fff;
        if addr < 0x2000 {
            self.read_mapper(addr)
        } else if addr < 0x3f00 {
            self.read_nametable((addr - 0x2000) & 0xfff)
        } else if addr < 0x4000 {
            self.read_palette((addr - 0x3f00) & 0x1f)
        } else {
            panic!("invalid ppu read access at 0x{:04x}", addr)
        }
    }

    fn write(&self, mut addr: u16, data: u8) {
        addr &= 0x3fff;
        if addr < 0x2000 {
            self.write_mapper(addr, data);
        } else if addr < 0x3f00 {
            self.write_nametable((addr - 0x2000) & 0xfff, data);
        } else if addr < 0x4000 {
            self.write_palette((addr - 0x3f00) & 0x1f, data);
        } else {
            panic!("invalid ppu write access at 0x{:04x}", addr)
        }
    }

}
