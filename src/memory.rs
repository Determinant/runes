#![allow(dead_code)]
use ppu::PPU;
use apu::{APU, Sampler};
use mos6502::{CPU, CPU_FREQ};
use cartridge::MirrorType;
use mapper::Mapper;
use controller::Controller;
use core::cell::RefCell;
use core::ptr::null_mut;

pub trait VMem {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

pub struct CPUBus<'a> {
    cpu: *mut CPU<'a>,
    ppu: *mut PPU<'a>,
    apu: *mut APU<'a>,
    ppu_sampler: RefCell<Sampler>,
}

impl<'a> CPUBus<'a> {
    pub fn new() -> Self {
        CPUBus {ppu: null_mut(),
                cpu: null_mut(),
                apu: null_mut(),
                ppu_sampler: RefCell::new(Sampler::new(CPU_FREQ, 60)),
            }
    }

    pub fn attach(&mut self, cpu: *mut CPU<'a>,
                             ppu: *mut PPU<'a>,
                             apu: *mut APU<'a>) {
        self.ppu = ppu;
        self.cpu = cpu;
        self.apu = apu;
    }

    #[inline(always)] pub fn get_cpu(&self) -> &'a mut CPU<'a> {unsafe{&mut *self.cpu}}
    #[inline(always)] pub fn get_ppu(&self) -> &'a mut PPU<'a> {unsafe{&mut *self.ppu}}
    #[inline(always)] pub fn get_apu(&self) -> &'a mut APU<'a> {unsafe{&mut *self.apu}}

    pub fn tick(&self) {
        let cpu = self.get_cpu();
        let ppu = self.get_ppu();
        let apu = self.get_apu();
        cpu.tick();
        if apu.tick(self) {
            cpu.trigger_irq()
        }
        if ppu.tick(self) || ppu.tick(self) || ppu.tick(self) {
            cpu.trigger_nmi()
        }
        if let (true, _) = self.ppu_sampler.borrow_mut().tick() {
            ppu.scr.frame()
        }
    }
}

pub struct CPUMemory<'a> {
    sram: [u8; 2048],
    pub bus: CPUBus<'a>,
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
                  bus: CPUBus::new(),
                  mapper, ctl1, ctl2}
    }

    pub fn get_bus(&'a self) -> &'a CPUBus<'a> {
        &self.bus
    }

    #[inline(always)]
    pub fn read_without_tick(&self, addr: u16) -> u8 {
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
            let apu = self.bus.get_apu();
            match addr {
                0x4015 => apu.read_status(),
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

    #[inline(always)]
    pub fn write_without_tick(&mut self, addr: u16, data: u8) {
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
            let apu = self.bus.get_apu();
            match addr {
                0x4000 => apu.pulse1.write_reg1(data),
                0x4001 => apu.pulse1.write_reg2(data),
                0x4002 => apu.pulse1.write_reg3(data),
                0x4003 => apu.pulse1.write_reg4(data),
                0x4004 => apu.pulse2.write_reg1(data),
                0x4005 => apu.pulse2.write_reg2(data),
                0x4006 => apu.pulse2.write_reg3(data),
                0x4007 => apu.pulse2.write_reg4(data),
                0x4008 => apu.triangle.write_reg1(data),
                0x400a => apu.triangle.write_reg3(data),
                0x400b => apu.triangle.write_reg4(data),
                0x400c => apu.noise.write_reg1(data),
                0x400e => apu.noise.write_reg3(data),
                0x400f => apu.noise.write_reg4(data),
                0x4010 => apu.dmc.write_reg1(data),
                0x4011 => apu.dmc.write_reg2(data),
                0x4012 => apu.dmc.write_reg3(data),
                0x4013 => apu.dmc.write_reg4(data),
                0x4015 => apu.write_status(data),
                0x4017 => apu.write_frame_counter(data),
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

impl<'a> VMem for CPUMemory<'a> {
    fn read(&self, addr: u16) -> u8 {
        self.bus.tick();
        self.read_without_tick(addr)
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.bus.tick();
        self.write_without_tick(addr, data);
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
        self.mapper.borrow_mut().write(addr, data)
    }

    #[inline(always)]
    pub fn tick(&self, bus: &CPUBus) {
        self.mapper.borrow_mut().tick(bus)
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
