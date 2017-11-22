#![allow(dead_code)]
use ppu::PPU;
use mos6502::CPU;
use cartridge::MirrorType;
use mapper::Mapper;
use controller::Controller;
use core::cell::RefCell;
use core::ptr::null_mut;

pub trait VMem {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

pub struct Bus<'a> {
    cpu: *mut CPU<'a>,
    ppu: *mut PPU<'a>,
}

impl<'a> Bus<'a> {
    pub fn new() -> Self {
        Bus {ppu: null_mut(),
            cpu: null_mut()}
    }

    pub fn attach(&mut self, cpu: *mut CPU<'a>, ppu: *mut PPU<'a>) {
        self.ppu = ppu;
        self.cpu = cpu;
    }

    #[inline(always)] fn get_cpu(&self) -> &'a mut CPU<'a> {unsafe{&mut *self.cpu}}
    #[inline(always)] fn get_ppu(&self) -> &'a mut PPU<'a> {unsafe{&mut *self.ppu}}
}

pub struct CPUMemory<'a> {
    sram: [u8; 2048],
    pub bus: Bus<'a>,
    mapper: &'a RefCell<&'a mut Mapper>,
    ctl1: Option<&'a Controller>,
    ctl2: Option<&'a Controller>
}

impl<'a> CPUMemory<'a> {
    pub fn new(
               mapper: &'a RefCell<&'a mut Mapper>,
               ctl1: Option<&'a Controller>,
               ctl2: Option<&'a Controller>) -> Self {
        CPUMemory{sram: [0; 2048],
                  bus: Bus::new(),
                  mapper, ctl1, ctl2}
    }

    pub fn ppu_tick(&self) {
        let cpu = self.bus.get_cpu();
        let ppu = self.bus.get_ppu();
        if ppu.tick() || ppu.tick() || ppu.tick() {
            cpu.trigger_nmi();
        }
        cpu.cycle -= 1;
    }
}

impl<'a> VMem for CPUMemory<'a> {
    fn read(&self, addr: u16) -> u8 {
        self.ppu_tick();
        let cpu = self.bus.get_cpu();
        let ppu = self.bus.get_ppu();
        if addr < 0x2000 {
            self.sram[(addr & 0x07ff) as usize]
        } else if addr < 0x4000 {
            match addr & 0x7 {
                0x2 => ppu.read_status(cpu),
                0x4 => ppu.read_oamdata(),
                0x7 => ppu.read_data(),
                _ => 0
            }
        } else if addr < 0x4020 {
            match addr {
                0x4016 => if let Some(c) = self.ctl1 { c.read() } else { 0 },
                0x4017 => if let Some(c) = self.ctl2 { c.read() } else { 0 },
                _ => 0
            }
        } else if addr < 0x6000 {
            0
        } else {
            self.mapper.borrow().read(addr)
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.ppu_tick();
        let cpu = self.bus.get_cpu();
        let ppu = self.bus.get_ppu();
        if addr < 0x2000 {
            self.sram[(addr & 0x07ff) as usize] = data;
        } else if addr < 0x4000 {
            match addr & 0x7 {
                0x0 => {
                    let old = ppu.get_flag_nmi();
                    ppu.write_ctl(data);
                    if !old && ppu.try_nmi() && ppu.vblank {
                        cpu.trigger_delayed_nmi()
                    } /* toggle NMI flag can generate multiple ints */
                },
                0x1 => ppu.write_mask(data),
                0x2 => (),
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
                0x4016 => {
                    if let Some(c) = self.ctl1 { c.write(data) }
                    if let Some(c) = self.ctl2 { c.write(data) }
                }
                _ => ()
            }
        } else if addr < 0x6000 {
        } else {
            self.mapper.borrow_mut().write(addr, data)
        }
    }
}

pub struct PPUMemory<'a> {
    nametable: [u8; 0x800],
    palette: [u8; 0x20],
    mapper: &'a RefCell<&'a mut Mapper>,
}

impl<'a> PPUMemory<'a> {
    pub fn new(mapper: &'a RefCell<&'a mut Mapper>) -> Self {
        PPUMemory{
            nametable: [0; 0x800],
            palette: [0; 0x20],
            mapper}
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
        let mt = self.mapper.borrow().get_cart().get_mirror_type();
        self.nametable[(get_mirror_addr(mt, addr) & 0x7ff) as usize]
    }

    #[inline(always)]
    pub fn read_palette(&self, addr: u16) -> u8 {
        self.palette[get_mirror_palette(addr) as usize]
    }

    #[inline(always)]
    pub fn write_nametable(&mut self, addr: u16, data: u8) {
        let mt = self.mapper.borrow().get_cart().get_mirror_type();
        self.nametable[(get_mirror_addr(mt, addr) & 0x7ff) as usize] = data
    }

    #[inline(always)]
    pub fn write_palette(&mut self, addr: u16, data: u8) {
        self.palette[get_mirror_palette(addr) as usize] = data
    }

    #[inline(always)]
    pub fn read_mapper(&self, addr: u16) -> u8 {
        self.mapper.borrow().read(addr)
    }

    #[inline(always)]
    fn write_mapper(&self, addr: u16, data: u8) {
        self.mapper.borrow_mut().write(addr, data);
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

    fn write(&mut self, mut addr: u16, data: u8) {
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
