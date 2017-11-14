#![allow(dead_code)]
use ppu::PPU;
use mos6502::CPU;
use cartridge::{MirrorType, Cartridge};
use core::cell::{RefCell, Cell};
use core::ptr::null_mut;

pub trait VMem {
    fn read(&self, addr: u16) -> u8;
    fn write(&self, addr: u16, data: u8);
}

pub struct CPUMemory<'a> {
    sram: RefCell<[u8; 2048]>,
    ppu: Cell<*mut PPU<'a>>,
    cpu: Cell<*mut CPU<'a>>,
    mapper: &'a VMem
}

impl<'a> CPUMemory<'a> {
    pub fn new(mapper: &'a VMem) -> Self {
        CPUMemory{sram: RefCell::new([0; 2048]),
                  cpu: Cell::new(null_mut()),
                  ppu: Cell::new(null_mut()),
                  mapper}
    }

    pub fn init(&self,
               cpu: *mut CPU<'a>,
               ppu: *mut PPU<'a>) {
        self.cpu.set(cpu);
        self.ppu.set(ppu);
    }
}

impl<'a> VMem for CPUMemory<'a> {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            _ => if addr < 0x2000 {
                self.sram.borrow()[(addr & 0x07ff) as usize]
            } else if addr < 0x4000 {
                let ppu = unsafe {&mut *self.ppu.get()};
                match addr & 0x7 {
                    0x2 => ppu.read_status(),
                    0x4 => ppu.read_oamdata(),
                    0x7 => ppu.read_data(),
                    _ => 0
                }
            } else if addr < 0x6000 {
                println!("feeding dummy data for 0x{:04x}", addr);
                0
            } else {
                self.mapper.read(addr)
            }
        }
    }
    fn write(&self, addr: u16, data: u8) {
            let ppu = unsafe {&mut *self.ppu.get()};
            let cpu = unsafe {&mut *self.cpu.get()};
            if addr < 0x2000 {
                self.sram.borrow_mut()[(addr & 0x07ff) as usize] = data;
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
                    _ => println!("ignore writing for 0x{:04x}", addr)
                }
            } else if addr < 0x6000 {
            } else {
                self.mapper.write(addr, data)
            }
    }
}

pub struct PPUMemory<'a> {
    pattern_table: RefCell<[u8; 0x2000]>,
    nametable: RefCell<[u8; 0x800]>,
    palette: RefCell<[u8; 0x20]>,
    cart: &'a Cartridge,
    mapper: &'a VMem,
}

impl<'a> PPUMemory<'a> {
    pub fn new(mapper: &'a VMem,
               cart: &'a Cartridge) -> Self {
        PPUMemory{
            pattern_table: RefCell::new([0; 0x2000]),
            nametable: RefCell::new([0; 0x800]),
            palette: RefCell::new([0; 0x20]),
            cart,
            mapper}
    }

    pub fn dump(&self) {
        for (i, v) in self.palette.borrow().iter().enumerate() {
            print!("{:02x} ", *v);
            if (i & 0x7) == 0x7 {println!("@{:02x}", i)}
        }
        for (i, v) in self.nametable.borrow().iter().enumerate() {
            print!("{:02x} ", *v);
            if (i & 0x1f) == 0x1f {println!("@{:02x}", i)}
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

fn get_mirror_addr(kind: MirrorType, mut addr: u16) -> u16 {
    addr = (addr - 0x2000) & 0x0fff;
    let table = addr >> 10;
    let offset = addr & 0x03ff;
    0x2000 + ((MIRROR_IDX[kind as usize][table as usize] as u16) << 10) + offset
}

fn mirror_palette(addr: u16) -> u16 {
    if addr >= 0x10 && addr & 3 == 0 {
        addr - 0x10
    } else { addr }
}

impl<'a> VMem for PPUMemory<'a> {
    fn read(&self, mut addr: u16) -> u8 {
        addr &= 0x3fff;
        if addr < 0x2000 {
            self.mapper.read(addr)
        } else if addr < 0x3f00 {
            let kind = self.cart.mirror_type;
            self.nametable.borrow()
                [(get_mirror_addr(kind, addr) & 0x07ff) as usize]
        } else if addr < 0x4000 {
            self.palette.borrow()
                [mirror_palette(addr & 0x1f) as usize]
        } else {
            panic!("invalid ppu read access at 0x{:04x}", addr)
        }
    }

    fn write(&self, mut addr: u16, data: u8) {
        addr &= 0x3fff;
        println!("writing 0x{:02x} to 0x{:04x}", data, addr);
        if addr < 0x2000 {
            self.mapper.write(addr, data)
        } else if addr < 0x3f00 {
            let kind = self.cart.mirror_type;
            self.nametable.borrow_mut()
                [(get_mirror_addr(kind, addr) & 0x07ff) as usize] = data
        } else if addr < 0x4000 {
            self.palette.borrow_mut()
                [mirror_palette(addr & 0x1f) as usize] = data
        } else {
            panic!("invalid ppu write access at 0x{:04x}", addr)
        }
    }

}
