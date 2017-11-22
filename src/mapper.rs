#![allow(dead_code)]
extern crate core;
use memory::VMem;
use cartridge::{Cartridge, BankType, MirrorType};

pub trait Mapper : VMem {
    fn get_cart(&self) -> &Cartridge;
}

pub struct Mapper1<'a, C> where C: Cartridge {
    cart: C,
    prg_banks: [&'a [u8]; 2],
    chr_banks: [&'a mut [u8]; 2],
    sram: &'a mut [u8],
    ctl_reg: u8,
    load_reg: u8,
    prg_nbank: usize, /* num of 16k PRG ROM banks */
    chr_nbank: usize /* num of 8k PRG ROM banks */
}

impl<'a, C> VMem for Mapper1<'a, C> where C: Cartridge {
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < 0x2000 {         /* 0x2000 size bank */
            self.chr_banks[(addr >> 12) & 1][addr & 0xfff]
        } else if addr >= 0x8000 { /* 0x4000 size bank */
            self.prg_banks[(addr >> 14) & 1][addr & 0x3fff]
        } else if addr >= 0x6000 {
            self.sram[addr - 0x6000]
        } else {
            panic!("unmapped address: 0x{:04x}", addr)
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        let addr = addr as usize;
        if addr < 0x2000 {
            self.chr_banks[(addr >> 12) & 1][addr & 0xfff] = data
        } else if addr >= 0x8000 {
            self.write_loadreg(addr as u16, data)
        } else if addr >= 0x6000 {
            self.sram[addr - 0x6000] = data
        } else {
            panic!("invalid write to address: 0x{:04x}", addr);
        }
    }
}

impl<'a, C> Mapper1<'a, C> where C: Cartridge {
    pub fn new(cart: C) -> Self {
        unsafe {
            let prg_nbank = cart.get_size(BankType::PrgRom) >> 14;
            let chr_nbank = cart.get_size(BankType::ChrRom) >> 13;
            let null = core::mem::uninitialized();
            let mut m = Mapper1{cart,
                        prg_nbank,
                        chr_nbank,
                        load_reg: 0x10,
                        ctl_reg: 0,
                        prg_banks: [null; 2],
                        chr_banks: [core::mem::uninitialized(),
                                    core::mem::uninitialized()],
                        sram: core::mem::uninitialized()};
            {
                let c = &mut m.cart;
                    m.prg_banks[0] = &*c.get_bank(0, 0x4000, BankType::PrgRom);
                    m.prg_banks[1] = &*c.get_bank(0x4000, 0x4000, BankType::PrgRom);
                    m.chr_banks[0] = &mut *c.get_bank(0, 0x1000, BankType::ChrRom);
                    m.chr_banks[1] = &mut *c.get_bank(0x1000, 0x1000, BankType::ChrRom);
                    m.sram = &mut *c.get_bank(0, 0x2000, BankType::Sram);
            }
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
        if triggered { unsafe {
            let load_reg = self.load_reg;
            match (addr >> 13) & 3 {
                0x0 => {
                    self.ctl_reg = load_reg;
                    self.cart.set_mirror_type(match load_reg & 3 {
                        0x0 => MirrorType::Single0,
                        0x1 => MirrorType::Single1,
                        0x2 => MirrorType::Vertical,
                        _ => MirrorType::Horizontal
                    });
                },
                0x1 => {
                    match (self.ctl_reg >> 4) & 1 {
                        0x0 => {
                            let base = ((load_reg & 0xfe) as usize % self.chr_nbank) << 13;
                            self.chr_banks[0] =
                                &mut *self.cart.get_bank(base, 0x1000, BankType::ChrRom);
                            self.chr_banks[1] =
                                &mut *self.cart.get_bank(base + 0x1000, 0x1000, BankType::ChrRom)
                        },
                        _ =>
                            self.chr_banks[0] = &mut *self.cart.get_bank(
                                (load_reg as usize % (self.chr_nbank << 1)) << 12,
                                0x1000, BankType::ChrRom)
                    }
                },
                0x2 => {
                    if (self.ctl_reg >> 4) & 1 == 1 {
                        self.chr_banks[1] = &mut *self.cart.get_bank(
                                (load_reg as usize % (self.chr_nbank << 1)) << 12,
                                0x1000, BankType::ChrRom)
                    }
                },
                0x3 => {
                    let load_reg = load_reg & 0xf;
                    match (self.ctl_reg >> 2) & 3 {
                        0x0 | 0x1 => {
                            let base = ((load_reg & 0xfe) as usize % (self.prg_nbank >> 1)) << 15;
                            self.prg_banks[0] =
                                &*self.cart.get_bank(base, 0x4000, BankType::PrgRom);
                            self.prg_banks[1] =
                                &*self.cart.get_bank(base + 0x4000, 0x4000, BankType::PrgRom)
                        },
                        0x2 => {
                            self.prg_banks[0] = &*self.cart.get_bank(0, 0x4000, BankType::PrgRom);
                            self.prg_banks[1] = &*self.cart.get_bank(
                                (load_reg as usize % self.prg_nbank) << 14,
                                0x4000, BankType::PrgRom);
                        },
                        0x3 => {
                            self.prg_banks[0] = &*self.cart.get_bank(
                                (load_reg as usize % self.prg_nbank) << 14,
                                0x4000, BankType::PrgRom);
                            self.prg_banks[1] = &*self.cart.get_bank(
                                (self.prg_nbank - 1) << 14,
                                0x4000, BankType::PrgRom);
                        }
                        _ => ()
                    }
                },
                _ => ()
            }
            self.load_reg = 0x10;
        }}
    }
}

impl<'a, C> Mapper for Mapper1<'a, C> where C: Cartridge {
    fn get_cart(&self) -> &Cartridge {&self.cart}
}

pub struct Mapper2<'a, C> where C: Cartridge {
    cart: C,
    prg_banks: [&'a [u8]; 2],
    chr_bank: &'a mut [u8],
    sram: &'a mut [u8],
    prg_nbank: usize
}

impl<'a, C> VMem for Mapper2<'a, C> where C: Cartridge {
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < 0x2000 {         /* 0x2000 size bank */
            self.chr_bank[addr]
        } else if addr >= 0x8000 { /* 0x4000 size bank */
            self.prg_banks[(addr >> 14) & 1][addr & 0x3fff]
        } else if addr >= 0x6000 {
            self.sram[addr - 0x6000]
        } else {
            panic!("unmapped address: 0x{:04x}", addr)
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        let addr = addr as usize;
        if addr < 0x2000 {
            self.chr_bank[addr] = data;
        } else if addr >= 0x8000 {
            self.prg_banks[0] = unsafe {
                &*self.cart.get_bank(((data as usize) % self.prg_nbank) << 14,
                0x4000,
                BankType::PrgRom)
            }
        } else if addr >= 0x6000 {
            self.sram[addr - 0x6000] = data
        } else {
            panic!("invalid write to address: 0x{:04x}", addr);
        }
    }
}

impl<'a, C> Mapper2<'a, C> where C: Cartridge {
    pub fn new(cart: C) -> Self {
        unsafe {
            let nbank = cart.get_size(BankType::PrgRom) >> 14;
            let null = core::mem::uninitialized();
            let mut m = Mapper2{cart,
                        prg_nbank: nbank,
                        prg_banks: [null; 2],
                        chr_bank: core::mem::uninitialized(),
                        sram: core::mem::uninitialized()};
            {
                let c = &mut m.cart;
                    m.prg_banks[0] = &*c.get_bank(0, 0x4000, BankType::PrgRom);
                    m.prg_banks[1] = &*c.get_bank((nbank - 1) << 14, 0x4000, BankType::PrgRom);
                    m.chr_bank = &mut *c.get_bank(0, 0x2000, BankType::ChrRom);
                    m.sram = &mut *c.get_bank(0, 0x2000, BankType::Sram);
            }
            m
        }
    }
}

impl<'a, C> Mapper for Mapper2<'a, C> where C: Cartridge {
    fn get_cart(&self) -> &Cartridge {&self.cart}
}
