use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

use crate::cartridge::{BankType, Cartridge, MirrorType};
use crate::memory::{CPUBus, VMem};
use crate::utils::{load_prefix, save_prefix, Read, Write};

pub trait Mapper: VMem {
    fn get_cart(&self) -> &dyn Cartridge;
    fn get_cart_mut(&mut self) -> &mut dyn Cartridge;
    fn tick(&mut self, _bus: &CPUBus) {}
    fn load(&mut self, reader: &mut dyn Read) -> bool;
    fn save(&self, writer: &mut dyn Write) -> bool;
}

pub struct RefMapper<'a> {
    mapper: UnsafeCell<&'a mut dyn Mapper>,
}

impl<'a> RefMapper<'a> {
    pub fn new(mapper: &'a mut dyn Mapper) -> Self {
        RefMapper {
            mapper: UnsafeCell::new(mapper),
        }
    }

    #[inline(always)]
    pub fn get_mut(&self) -> &'a mut dyn Mapper {
        unsafe { *self.mapper.get() }
    }
}

impl<'a> core::ops::Deref for RefMapper<'a> {
    type Target = &'a mut dyn Mapper;
    #[inline(always)]
    fn deref(&self) -> &&'a mut dyn Mapper {
        unsafe { &*self.mapper.get() }
    }
}

#[repr(C)]
pub struct Mapper1<'a, C>
where
    C: Cartridge,
{
    cart: C,
    prg_banks: [&'a [u8]; 2],
    chr_banks: [&'a mut [u8]; 2],
    sram: &'a mut [u8],
    ctl_reg: u8,
    load_reg: u8,
    prg_nbank: usize, /* num of 16k PRG ROM banks */
    chr_nbank: usize, /* num of 8k PRG ROM banks */
}

impl<'a, C> VMem for Mapper1<'a, C>
where
    C: Cartridge,
{
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.chr_banks[(addr >> 12) & 1][addr & 0xfff],
            /* [0x2000..0x6000) */
            2 | 3 | 4 | 5 => panic!("unmapped address: 0x{:04x}", addr),
            /* [0x6000..0x8000) */
            6 | 7 => self.sram[addr - 0x6000],
            /* [0x8000..0xffff] */
            _ => self.prg_banks[(addr >> 14) & 1][addr & 0x3fff],
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        let addr = addr as usize;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.chr_banks[(addr >> 12) & 1][addr & 0xfff] = data,
            /* [0x2000..0x6000) */
            2 | 3 | 4 | 5 => panic!("unmapped address: 0x{:04x}", addr),
            /* [0x6000..0x8000) */
            6 | 7 => self.sram[addr - 0x6000] = data,
            /* [0x8000..0xffff] */
            _ => self.write_loadreg(addr as u16, data),
        }
    }
}

impl<'a, C> Mapper1<'a, C>
where
    C: Cartridge,
{
    pub fn new(cart: C) -> Self {
        let prg_nbank = cart.get_size(BankType::PrgRom) >> 14;
        let chr_nbank = cart.get_size(BankType::ChrRom) >> 13;
        unsafe {
            let mut m = Mapper1 {
                cart,
                prg_nbank,
                chr_nbank,
                load_reg: 0x10,
                ctl_reg: 0x0c,
                prg_banks: MaybeUninit::uninit().assume_init(),
                chr_banks: MaybeUninit::uninit().assume_init(),
                sram: core::slice::from_raw_parts_mut(core::ptr::null_mut(), 0),
            };
            let c = &mut m.cart;
            m.prg_banks = [
                c.get_bank(0, 0x4000, BankType::PrgRom),
                c.get_bank((prg_nbank - 1) << 14, 0x4000, BankType::PrgRom),
            ];
            m.chr_banks = [
                c.get_bank_mut(0, 0x1000, BankType::ChrRom),
                c.get_bank_mut(0x1000, 0x1000, BankType::ChrRom),
            ];
            m.sram = c.get_bank_mut(0, 0x2000, BankType::Sram);
            m
        }
    }

    fn write_loadreg(&mut self, addr: u16, data: u8) {
        if data & 0x80 == 0x80 {
            self.ctl_reg |= 0x0c;
            self.load_reg = 0x10;
            return
        }
        let triggered = self.load_reg & 1 == 1;
        self.load_reg = (self.load_reg >> 1) | ((data & 1) << 4);
        if !triggered {
            return
        }

        let load_reg = self.load_reg;
        match (addr >> 13) & 3 {
            0x0 => {
                self.ctl_reg = load_reg;
                self.cart.set_mirror_type(match load_reg & 3 {
                    0x0 => MirrorType::Single0,
                    0x1 => MirrorType::Single1,
                    0x2 => MirrorType::Vertical,
                    _ => MirrorType::Horizontal,
                });
            }
            0x1 => match (self.ctl_reg >> 4) & 1 {
                0x0 => {
                    let base =
                        ((load_reg & 0xfe) as usize % self.chr_nbank) << 13;
                    self.chr_banks = [
                        self.cart.get_bank_mut(base, 0x1000, BankType::ChrRom),
                        self.cart.get_bank_mut(
                            base + 0x1000,
                            0x1000,
                            BankType::ChrRom,
                        ),
                    ];
                }
                _ => {
                    self.chr_banks[0] = self.cart.get_bank_mut(
                        (load_reg as usize % (self.chr_nbank << 1)) << 12,
                        0x1000,
                        BankType::ChrRom,
                    )
                }
            },
            0x2 => {
                if (self.ctl_reg >> 4) & 1 == 1 {
                    self.chr_banks[1] = self.cart.get_bank_mut(
                        (load_reg as usize % (self.chr_nbank << 1)) << 12,
                        0x1000,
                        BankType::ChrRom,
                    )
                }
            }
            0x3 => {
                let load_reg = load_reg & 0xf;
                match (self.ctl_reg >> 2) & 3 {
                    0x0 | 0x1 => {
                        let base = ((load_reg & 0xfe) as usize %
                            (self.prg_nbank >> 1)) <<
                            15;
                        self.prg_banks = [
                            self.cart.get_bank(base, 0x4000, BankType::PrgRom),
                            self.cart.get_bank(
                                base + 0x4000,
                                0x4000,
                                BankType::PrgRom,
                            ),
                        ];
                    }
                    0x2 => {
                        self.prg_banks = [
                            self.cart.get_bank(0, 0x4000, BankType::PrgRom),
                            self.cart.get_bank(
                                (load_reg as usize % self.prg_nbank) << 14,
                                0x4000,
                                BankType::PrgRom,
                            ),
                        ]
                    }
                    0x3 => {
                        self.prg_banks = [
                            self.cart.get_bank(
                                (load_reg as usize % self.prg_nbank) << 14,
                                0x4000,
                                BankType::PrgRom,
                            ),
                            self.cart.get_bank(
                                (self.prg_nbank - 1) << 14,
                                0x4000,
                                BankType::PrgRom,
                            ),
                        ]
                    }
                    _ => (),
                }
            }
            _ => (),
        }
        self.load_reg = 0x10;
    }
}

impl<'a, C> Mapper for Mapper1<'a, C>
where
    C: Cartridge,
{
    fn get_cart(&self) -> &dyn Cartridge {
        &self.cart
    }
    fn get_cart_mut(&mut self) -> &mut dyn Cartridge {
        &mut self.cart
    }

    fn load(&mut self, reader: &mut dyn Read) -> bool {
        for v in self.prg_banks.iter_mut() {
            let mut offset: usize = 0;
            if !load_prefix(&mut offset, 0, reader) {
                return false
            }
            *v = self.cart.get_bank(offset, 0x4000, BankType::PrgRom);
        }
        for v in self.chr_banks.iter_mut() {
            let mut offset: usize = 0;
            if !load_prefix(&mut offset, 0, reader) {
                return false
            }
            *v = self.cart.get_bank_mut(offset, 0x1000, BankType::ChrRom);
        }
        load_prefix(&mut self.ctl_reg, 0, reader) &&
            load_prefix(&mut self.load_reg, 0, reader) &&
            self.cart.load(reader)
    }

    fn save(&self, writer: &mut dyn Write) -> bool {
        let prg_base = self.cart.get_bank(0, 0, BankType::PrgRom).as_ptr();
        let chr_base = self.cart.get_bank(0, 0, BankType::ChrRom).as_ptr();
        for v in self.prg_banks.iter() {
            if !save_prefix(
                &(v.as_ptr() as usize - prg_base as usize),
                0,
                writer,
            ) {
                return false
            }
        }
        for v in self.chr_banks.iter() {
            if !save_prefix(
                &(v.as_ptr() as usize - chr_base as usize),
                0,
                writer,
            ) {
                return false
            }
        }
        save_prefix(&self.ctl_reg, 0, writer) &&
            save_prefix(&self.load_reg, 0, writer) &&
            self.cart.save(writer)
    }
}

#[repr(C)]
pub struct Mapper2<'a, C>
where
    C: Cartridge,
{
    cart: C,
    prg_banks: [&'a [u8]; 2],
    chr_bank: &'a mut [u8],
    sram: &'a mut [u8],
    prg_nbank: usize,
}

impl<'a, C> VMem for Mapper2<'a, C>
where
    C: Cartridge,
{
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.chr_bank[addr],
            /* [0x2000..0x6000) */
            2 | 3 | 4 | 5 => panic!("unmapped address: 0x{:04x}", addr),
            /* [0x6000..0x8000) */
            6 | 7 => self.sram[addr - 0x6000],
            /* [0x8000..0xffff] */
            _ => self.prg_banks[(addr >> 14) & 1][addr & 0x3fff],
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        let addr = addr as usize;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.chr_bank[addr] = data,
            /* [0x2000..0x6000) */
            2 | 3 | 4 | 5 => panic!("unmapped address: 0x{:04x}", addr),
            /* [0x6000..0x8000) */
            6 | 7 => self.sram[addr - 0x6000] = data,
            /* [0x8000..0xffff] */
            _ => {
                self.prg_banks[0] = self.cart.get_bank(
                    ((data as usize) % self.prg_nbank) << 14,
                    0x4000,
                    BankType::PrgRom,
                )
            }
        }
    }
}

impl<'a, C> Mapper2<'a, C>
where
    C: Cartridge,
{
    pub fn new(cart: C) -> Self {
        let nbank = cart.get_size(BankType::PrgRom) >> 14;
        unsafe {
            let mut m = Mapper2 {
                cart,
                prg_nbank: nbank,
                prg_banks: MaybeUninit::uninit().assume_init(),
                chr_bank: core::slice::from_raw_parts_mut(
                    core::ptr::null_mut(),
                    0,
                ),
                sram: core::slice::from_raw_parts_mut(core::ptr::null_mut(), 0),
            };
            let c = &mut m.cart;
            m.prg_banks = [
                c.get_bank(0, 0x4000, BankType::PrgRom),
                c.get_bank((nbank - 1) << 14, 0x4000, BankType::PrgRom),
            ];
            m.chr_bank = c.get_bank_mut(0, 0x2000, BankType::ChrRom);
            m.sram = c.get_bank_mut(0, 0x2000, BankType::Sram);
            m
        }
    }
}

impl<'a, C> Mapper for Mapper2<'a, C>
where
    C: Cartridge,
{
    fn get_cart(&self) -> &dyn Cartridge {
        &self.cart
    }
    fn get_cart_mut(&mut self) -> &mut dyn Cartridge {
        &mut self.cart
    }

    fn load(&mut self, reader: &mut dyn Read) -> bool {
        for v in self.prg_banks.iter_mut() {
            let mut offset: usize = 0;
            if !load_prefix(&mut offset, 0, reader) {
                return false
            }
            *v = self.cart.get_bank(offset, 0x4000, BankType::PrgRom);
        }
        let mut offset: usize = 0;
        if !load_prefix(&mut offset, 0, reader) {
            return false
        }
        self.chr_bank =
            self.cart.get_bank_mut(offset, 0x2000, BankType::ChrRom);
        self.cart.load(reader)
    }

    fn save(&self, writer: &mut dyn Write) -> bool {
        let prg_base = self.cart.get_bank(0, 0, BankType::PrgRom).as_ptr();
        let chr_base = self.cart.get_bank(0, 0, BankType::ChrRom).as_ptr();
        for v in self.prg_banks.iter() {
            if !save_prefix(
                &(v.as_ptr() as usize - prg_base as usize),
                0,
                writer,
            ) {
                return false
            }
        }
        if !save_prefix(
            &(self.chr_bank.as_ptr() as usize - chr_base as usize),
            0,
            writer,
        ) {
            return false
        }
        self.cart.save(writer)
    }
}

#[repr(C)]
pub struct Mapper4<'a, C>
where
    C: Cartridge,
{
    cart: C,
    prg_banks: [&'a [u8]; 4],
    chr_banks: [&'a mut [u8]; 8],
    sram: &'a mut [u8],
    prg_nbank: usize, /* num of 16k PRG ROM banks */
    chr_nbank: usize, /* num of 8k PRG ROM banks */
    chr_inv: u8,
    prg_mode: u8,
    reg_idx: u8,
    regs: [u8; 8], /* 4 pairs of registers */
    irq_reload: u8,
    irq_counter: u8,
    irq_enable: bool,
}

impl<'a, C> VMem for Mapper4<'a, C>
where
    C: Cartridge,
{
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.chr_banks[addr >> 10][addr & 0x3ff],
            /* [0x2000..0x6000) */
            2 | 3 | 4 | 5 => panic!("unmapped address: 0x{:04x}", addr),
            /* [0x6000..0x8000) */
            6 | 7 => self.sram[addr - 0x6000],
            /* [0x8000..0xffff] */
            _ => {
                let addr = addr - 0x8000;
                self.prg_banks[addr >> 13][addr & 0x1fff]
            }
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        let addr = addr as usize;
        match addr >> 12 {
            /* [0x0000..0x2000) */
            0 | 1 => self.chr_banks[addr >> 10][addr & 0x3ff] = data,
            /* [0x2000..0x6000) */
            2 | 3 | 4 | 5 => panic!("unmapped address: 0x{:04x}", addr),
            /* [0x6000..0x8000) */
            6 | 7 => self.sram[addr - 0x6000] = data,
            /* [0x8000..0xa000) */
            8 | 9 => match addr & 1 {
                0 => self.write_select_reg(data),
                _ => self.write_bank_data(data),
            },
            /* [0xa000..0xc000) */
            0xa | 0xb => match addr & 1 {
                0 => self.write_mirror(data),
                _ => (),
            },
            /* [0xc000..0xe000) */
            0xc | 0xd => match addr & 1 {
                0 => self.irq_reload = data,
                _ => self.irq_counter = 0,
            },
            /* [0xe000..0xffff] */
            _ => match addr & 1 {
                0 => self.irq_enable = false,
                _ => self.irq_enable = true,
            },
        }
    }
}

impl<'a, C> Mapper4<'a, C>
where
    C: Cartridge,
{
    #[inline(always)]
    fn write_select_reg(&mut self, data: u8) {
        self.prg_mode = (data >> 6) & 1;
        self.chr_inv = (data >> 7) & 1;
        self.reg_idx = data & 0x7;
        self.update_banks();
    }

    #[inline(always)]
    fn write_bank_data(&mut self, data: u8) {
        self.regs[self.reg_idx as usize] = data;
        self.update_banks();
    }

    #[inline(always)]
    fn write_mirror(&mut self, data: u8) {
        self.cart.set_mirror_type(match data & 1 {
            0 => MirrorType::Vertical,
            _ => MirrorType::Horizontal,
        })
    }

    #[inline(always)]
    fn get_prgbank<'b>(&self, idx: u8) -> &'b [u8] {
        self.cart
            .get_bank(0x2000 * idx as usize, 0x2000, BankType::PrgRom)
    }

    #[inline(always)]
    fn get_chrbank<'b>(&mut self, idx: u8) -> &'b mut [u8] {
        self.cart
            .get_bank_mut(0x400 * idx as usize, 0x400, BankType::ChrRom)
    }

    fn update_banks(&mut self) {
        macro_rules! make_arr {
            ($mt: ident, $m: ident, [$($x: expr), *]) => {
                [$({let t = ($x as usize % self.$m) as u8;
                    self.$mt(t)}), *]
            };
        }
        self.prg_banks = match self.prg_mode {
            0 => make_arr!(
                get_prgbank,
                prg_nbank,
                [
                    self.regs[6],
                    self.regs[7],
                    (self.prg_nbank - 2) as u8,
                    (self.prg_nbank - 1) as u8
                ]
            ),
            _ => make_arr!(
                get_prgbank,
                prg_nbank,
                [
                    (self.prg_nbank - 2) as u8,
                    self.regs[7],
                    self.regs[6],
                    (self.prg_nbank - 1) as u8
                ]
            ),
        };
        self.chr_banks = match self.chr_inv {
            0 => make_arr!(
                get_chrbank,
                chr_nbank,
                [
                    self.regs[0] & 0xfe,
                    self.regs[0] | 0x01,
                    self.regs[1] & 0xfe,
                    self.regs[1] | 0x01,
                    self.regs[2],
                    self.regs[3],
                    self.regs[4],
                    self.regs[5]
                ]
            ),
            _ => make_arr!(
                get_chrbank,
                chr_nbank,
                [
                    self.regs[2],
                    self.regs[3],
                    self.regs[4],
                    self.regs[5],
                    self.regs[0] & 0xfe,
                    self.regs[0] | 0x01,
                    self.regs[1] & 0xfe,
                    self.regs[1] | 0x01
                ]
            ),
        };
    }

    pub fn new(cart: C) -> Self {
        let prg_nbank = cart.get_size(BankType::PrgRom) >> 13;
        let chr_nbank = cart.get_size(BankType::ChrRom) >> 10;
        unsafe {
            let mut m = Mapper4 {
                cart,
                prg_nbank,
                chr_nbank,
                prg_mode: 0,
                chr_inv: 0,
                reg_idx: 0,
                regs: [0; 8],
                prg_banks: MaybeUninit::uninit().assume_init(),
                chr_banks: MaybeUninit::uninit().assume_init(),
                sram: core::slice::from_raw_parts_mut(core::ptr::null_mut(), 0),
                irq_reload: 0,
                irq_counter: 0,
                irq_enable: false,
            };
            m.prg_banks = [
                m.get_prgbank(0),
                m.get_prgbank(1),
                m.get_prgbank((prg_nbank - 2) as u8),
                m.get_prgbank((prg_nbank - 1) as u8),
            ];
            m.chr_banks = [
                m.get_chrbank(0),
                m.get_chrbank(0),
                m.get_chrbank(0),
                m.get_chrbank(0),
                m.get_chrbank(0),
                m.get_chrbank(0),
                m.get_chrbank(0),
                m.get_chrbank(0),
            ];
            let c = &mut m.cart;
            m.sram = c.get_bank_mut(0, 0x2000, BankType::Sram);
            m
        }
    }
}

impl<'a, C> Mapper for Mapper4<'a, C>
where
    C: Cartridge,
{
    fn get_cart(&self) -> &dyn Cartridge {
        &self.cart
    }
    fn get_cart_mut(&mut self) -> &mut dyn Cartridge {
        &mut self.cart
    }

    fn tick(&mut self, bus: &CPUBus) {
        let ppu = bus.get_ppu();
        if ppu.cycle != 260 {
            return
        }
        if ppu.scanline > 239 && ppu.scanline < 261 {
            return
        }
        if !ppu.get_show_bg() && !ppu.get_show_sp() {
            return
        }
        if self.irq_counter == 0 {
            self.irq_counter = self.irq_reload
        } else {
            self.irq_counter -= 1;
            if self.irq_counter == 0 && self.irq_enable {
                bus.get_cpu().trigger_irq();
            }
        }
    }

    fn load(&mut self, reader: &mut dyn Read) -> bool {
        for v in self.prg_banks.iter_mut() {
            let mut offset: usize = 0;
            if !load_prefix(&mut offset, 0, reader) {
                return false
            }
            *v = self.cart.get_bank(offset, 0x2000, BankType::PrgRom);
        }
        for v in self.chr_banks.iter_mut() {
            let mut offset: usize = 0;
            if !load_prefix(&mut offset, 0, reader) {
                return false
            }
            *v = self.cart.get_bank_mut(offset, 0x400, BankType::ChrRom);
        }
        load_prefix(&mut self.chr_inv, 0, reader) &&
            load_prefix(&mut self.prg_mode, 0, reader) &&
            load_prefix(&mut self.reg_idx, 0, reader) &&
            load_prefix(&mut self.regs, 0, reader) &&
            load_prefix(&mut self.irq_reload, 0, reader) &&
            load_prefix(&mut self.irq_counter, 0, reader) &&
            load_prefix(&mut self.irq_enable, 0, reader) &&
            self.cart.load(reader)
    }

    fn save(&self, writer: &mut dyn Write) -> bool {
        let prg_base = self.cart.get_bank(0, 0, BankType::PrgRom).as_ptr();
        let chr_base = self.cart.get_bank(0, 0, BankType::ChrRom).as_ptr();
        for v in self.prg_banks.iter() {
            if !save_prefix(
                &(v.as_ptr() as usize - prg_base as usize),
                0,
                writer,
            ) {
                return false
            }
        }
        for v in self.chr_banks.iter() {
            if !save_prefix(
                &(v.as_ptr() as usize - chr_base as usize),
                0,
                writer,
            ) {
                return false
            }
        }
        save_prefix(&self.chr_inv, 0, writer) &&
            save_prefix(&self.prg_mode, 0, writer) &&
            save_prefix(&self.reg_idx, 0, writer) &&
            save_prefix(&self.regs, 0, writer) &&
            save_prefix(&self.irq_reload, 0, writer) &&
            save_prefix(&self.irq_counter, 0, writer) &&
            save_prefix(&self.irq_enable, 0, writer) &&
            self.cart.save(writer)
    }
}
