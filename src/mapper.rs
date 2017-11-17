#![allow(dead_code)]
extern crate core;
use memory::VMem;
use cartridge::{Cartridge, BankType};

pub struct Mapper2<'a> {
    cart: Cartridge,
    prg_bank1: &'a [u8],
    prg_bank2: &'a [u8],
    chr_bank: &'a mut [u8],
    sram: &'a mut [u8],
    prg_nbank: usize
}

impl<'a> VMem for Mapper2<'a> {
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < 0x2000 {         /* 0x2000 size bank */
            self.chr_bank[addr]
        } else if addr >= 0xc000 { /* 0x4000 size bank */
            self.prg_bank2[addr - 0xc000]
        } else if addr >= 0x8000 { /* 0x4000 size bank */
            self.prg_bank1[addr - 0x8000]
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
            self.prg_bank1 = unsafe {
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

impl<'a> Mapper2<'a> {
    pub fn new(cart: Cartridge) -> Self {
        unsafe {
            let nbank = cart.get_size(BankType::PrgRom) >> 14;
            let null = core::mem::uninitialized();
            let mut m = Mapper2{cart,
                        prg_nbank: nbank,
                        prg_bank1: null,
                        prg_bank2: null,
                        chr_bank: core::mem::uninitialized(),
                        sram: core::mem::uninitialized()};
            {
                let c = &mut m.cart;
                    m.prg_bank1 = &*c.get_bank(0, 0x4000, BankType::PrgRom);
                    m.prg_bank2 = &*c.get_bank((nbank - 1) << 14, 0x4000, BankType::PrgRom);
                    m.chr_bank = &mut *c.get_bank(0, 0x2000, BankType::ChrRom);
                    m.sram = &mut *c.get_bank(0, 0x2000, BankType::Sram);
            }
            m
        }
    }

    pub fn get_cart(&self) -> &Cartridge {&self.cart}
}
