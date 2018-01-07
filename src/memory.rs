#![allow(dead_code)]
use ppu::PPU;
use apu::APU;
use utils::{Sampler, Read, Write, load_prefix, save_prefix};
use mos6502::{CPU, CPU_FREQ};
use cartridge::MirrorType;
use mapper::RefMapper;
use controller::Controller;
use core::cell::{RefCell, Cell};
use core::ptr::null_mut;
use core::mem::size_of;

pub trait VMem {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

#[repr(C)]
pub struct CPUBus<'a> {
    /*-- begin state --*/
    nmi_after_tick: Cell<bool>,
    cpu_stall: Cell<u32>,
    /*-- end state --*/

    /*-- begin sub-state --*/
    ppu_sampler: RefCell<Sampler>,
    /*-- end sub-state --*/

    cpu: *mut CPU<'a>,
    ppu: *mut PPU<'a>,
    apu: *mut APU<'a>,
}

const CPUBUS_IGNORED_SIZE: usize =
    size_of::<RefCell<Sampler>>() +
    size_of::<*mut CPU>() +
    size_of::<*mut PPU>() +
    size_of::<*mut APU>();

impl<'a> CPUBus<'a> {
    pub fn new() -> Self {
        CPUBus {ppu: null_mut(),
                cpu: null_mut(),
                apu: null_mut(),
                ppu_sampler: RefCell::new(Sampler::new(CPU_FREQ, 60)),
                nmi_after_tick: Cell::new(false),
                cpu_stall: Cell::new(0)
            }
    }

    pub fn load(&mut self, reader: &mut Read) -> bool {
        load_prefix(self, CPUBUS_IGNORED_SIZE, reader) &&
        self.ppu_sampler.borrow_mut().load(reader)
    }

    pub fn save(&self, writer: &mut Write) -> bool {
        save_prefix(self, CPUBUS_IGNORED_SIZE, writer) &&
        self.ppu_sampler.borrow().save(writer)
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

    pub fn cpu_stall(&self, delta: u32) {
        self.cpu_stall.set(self.cpu_stall.get() + delta)
    }

    pub fn tick(&self) {
        let cpu = self.get_cpu();
        let ppu = self.get_ppu();
        let apu = self.get_apu();

        let cpu_stall = self.cpu_stall.get();
        if cpu_stall == 0 {
            cpu.tick()
        } else {
            self.cpu_stall.set(cpu_stall - 1)
        }
        if apu.tick(self) {
            cpu.trigger_irq()
        }

        let first = ppu.tick(self);
        let second = ppu.tick(self);
        let third = ppu.tick(self);
        let mut nmi_after_tick = false;

        if first || second || third {
            nmi_after_tick = !first;
            if cpu.cycle == 0 && nmi_after_tick {
                cpu.trigger_delayed_nmi()
            } else {
                cpu.trigger_nmi()
            }
            //println!("nmi");
        }
        self.nmi_after_tick.set(nmi_after_tick);
        //println!("tick {} {}", ppu.scanline, ppu.cycle);
        if self.ppu_sampler.borrow_mut().tick() {
            ppu.scr.frame()
        }
    }
}

#[repr(C)]
pub struct CPUMemory<'a> {
    /*-- begin state --*/
    sram: [u8; 2048],
    /*-- end state --*/

    /*-- begin sub-state --*/
    pub bus: CPUBus<'a>,
    /*-- end sub-state --*/

    mapper: &'a RefMapper<'a>,
    ctl1: Option<&'a Controller>,
    ctl2: Option<&'a Controller>
}

const CPUMEM_IGNORED_SIZE: usize =
    size_of::<CPUBus>() +
    size_of::<&RefMapper>() +
    size_of::<Option<&Controller>>() +
    size_of::<Option<&Controller>>();

impl<'a> CPUMemory<'a> {
    pub fn new(mapper: &'a RefMapper<'a>,
               ctl1: Option<&'a Controller>,
               ctl2: Option<&'a Controller>) -> Self {
        CPUMemory{sram: [0; 2048],
                  bus: CPUBus::new(),
                  mapper, ctl1, ctl2}
    }

    pub fn load(&mut self, reader: &mut Read) -> bool {
        load_prefix(self, CPUMEM_IGNORED_SIZE, reader) &&
        self.bus.load(reader)
    }

    pub fn save(&self, writer: &mut Write) -> bool {
        save_prefix(self, CPUMEM_IGNORED_SIZE, writer) &&
        self.bus.save(writer)
    }

    pub fn get_bus(&'a self) -> &'a CPUBus<'a> {
        &self.bus
    }

    #[inline(always)]
    pub fn read_without_tick(&self, addr: u16) -> u8 {
        let cpu = self.bus.get_cpu();
        let ppu = self.bus.get_ppu();
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.sram[(addr & 0x07ff) as usize],
            /* [0x2000..0x4000) */
            2 | 3 => {
                match addr & 0x7 {
                    0x2 => {
                        if ppu.cycle == 2 || ppu.cycle == 3 {
                            cpu.suppress_nmi()
                        } /* race condition when status is read near vbl/nmi */
                        ppu.read_status()
                    },
                    0x4 => ppu.read_oamdata(),
                    0x7 => ppu.read_data(),
                    _ => 0
                }
            },
            /* [0x4000..0x5000) */
            4 => {
                let apu = self.bus.get_apu();
                match addr {
                    0x4015 => apu.read_status(),
                    0x4016 => if let Some(c) = self.ctl1 { c.read() } else { 0 },
                    0x4017 => if let Some(c) = self.ctl2 { c.read() } else { 0 },
                    _ => 0
                }
            },
            /* [0x5000..0x6000) */
            5 => 0,
            /* [0x6000..0xffff) */
            _ => self.mapper.read(addr)
        }
    }

    #[inline(always)]
    pub fn write_without_tick(&mut self, addr: u16, data: u8) {
        let cpu = self.bus.get_cpu();
        let ppu = self.bus.get_ppu();
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.sram[(addr & 0x07ff) as usize] = data,
            /* [0x2000..0x4000) */
            2 | 3 => match addr & 0x7 {
                0x0 => {
                    let old = ppu.get_flag_nmi();
                    ppu.write_ctl(data);
                    if !ppu.try_nmi() && self.bus.nmi_after_tick.get() {
                        cpu.suppress_nmi()
                    } /* NMI could be suppressed if disabled near set */
                    if !old && ppu.try_nmi() && ppu.vblank_lines {
                        cpu.trigger_delayed_nmi()
                    } /* toggle NMI flag can generate multiple ints */
                },
                0x1 => ppu.write_mask(data),
                0x2 => (),
                0x3 => ppu.write_oamaddr(data),
                0x4 => ppu.write_oamdata(data),
                0x5 => ppu.write_scroll(data),
                0x6 => ppu.write_addr(data),
                _ => ppu.write_data(data),
            },
            /* [0x4000..0x5000) */
            4 => {
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
                    0x4014 => ppu.write_oamdma(data, &self.bus),
                    0x4016 => {
                        if let Some(c) = self.ctl1 { c.write(data) }
                        if let Some(c) = self.ctl2 { c.write(data) }
                    }
                    _ => ()
                }
            },
            /* [0x5000..0x6000) */
            5 => (),
            /* [0x6000..0xffff) */
            _ => self.mapper.get_mut().write(addr, data)
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

#[repr(C)]
pub struct PPUMemory<'a> {
    /*-- begin state -- */
    nametable: [u8; 0x800],
    palette: [u8; 0x20],
    /*-- end state --*/

    mapper: &'a RefMapper<'a>
}

const PPUMEM_IGNORED_SIZE: usize = size_of::<&RefMapper>();

impl<'a> PPUMemory<'a> {
    pub fn new(mapper: &'a RefMapper<'a>) -> Self {
        PPUMemory{
            nametable: [0; 0x800],
            palette: [0; 0x20],
            mapper
        }
    }

    pub fn load(&mut self, reader: &mut Read) -> bool {
        load_prefix(self, PPUMEM_IGNORED_SIZE, reader)
    }

    pub fn save(&self, writer: &mut Write) -> bool {
        save_prefix(self, PPUMEM_IGNORED_SIZE, writer)
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
        let mt = self.mapper.get_cart().get_mirror_type();
        self.nametable[(get_mirror_addr(mt, addr) & 0x7ff) as usize]
    }

    #[inline(always)]
    pub fn read_palette(&self, addr: u16) -> u8 {
        self.palette[get_mirror_palette(addr) as usize]
    }

    #[inline(always)]
    pub fn write_nametable(&mut self, addr: u16, data: u8) {
        let mt = self.mapper.get_cart().get_mirror_type();
        self.nametable[(get_mirror_addr(mt, addr) & 0x7ff) as usize] = data
    }

    #[inline(always)]
    pub fn write_palette(&mut self, addr: u16, data: u8) {
        self.palette[get_mirror_palette(addr) as usize] = data
    }

    #[inline(always)]
    pub fn read_mapper(&self, addr: u16) -> u8 {
        self.mapper.read(addr)
    }

    #[inline(always)]
    fn write_mapper(&self, addr: u16, data: u8) {
        self.mapper.get_mut().write(addr, data)
    }

    #[inline(always)]
    pub fn tick(&self, bus: &CPUBus) {
        self.mapper.get_mut().tick(bus)
    }
}

impl<'a> VMem for PPUMemory<'a> {
    fn read(&self, mut addr: u16) -> u8 {
        addr &= 0x3fff;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.read_mapper(addr),
            /* [0x2000..0x3000) */
            2 => self.read_nametable((addr - 0x2000) & 0xfff),
            /* [0x3000..0x4000) */
            _ => match addr >> 8 {
                0x3f => self.read_palette((addr - 0x3f00) & 0x1f),
                _ => self.read_nametable((addr - 0x2000) & 0xfff)
            },
        }
    }

    fn write(&mut self, mut addr: u16, data: u8) {
        addr &= 0x3fff;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.write_mapper(addr, data),
            /* [0x2000..0x3000) */
            2 => self.write_nametable((addr - 0x2000) & 0xfff, data),
            /* [0x3000..0x4000) */
            _ => match addr >> 8 {
                0x3f => self.write_palette((addr - 0x3f00) & 0x1f, data),
                _ => self.write_nametable((addr - 0x2000) & 0xfff, data)
            },
        }
    }
}
