#![allow(dead_code)]
use memory::VMem;
use cartridge::{Cartridge, BankType};
pub struct Mapper2<'a> {
    cart: &'a Cartridge,
    prg_bank1: &'a [u8],
    prg_bank2: &'a [u8],
    chr_bank: &'a mut [u8],
    sram: &'a mut [u8],
    bank_num: usize
}

impl<'a> VMem for Mapper2<'a> {
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < 0x2000 {
            self.chr_bank[addr]
        } else if addr >= 0xc000 {
            self.prg_bank2[addr - 0xc000]
        } else if addr >= 0x8000 {
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
            self.prg_bank1 = unsafe {&*self.cart.get_bank(
                data as usize % self.bank_num, BankType::PrgRom)};
        } else if addr >= 0x6000 {
            self.sram[addr - 0x6000] = data;
        } else {
            panic!("invalid write to address: 0x{:04x}", addr);
        }
    }
}

impl<'a> Mapper2<'a> {
    pub fn new(cart: &'a mut Cartridge) -> Self {
        let bank_num = cart.get_bank_num(BankType::PrgRom);
        unsafe {
            Mapper2{cart,
                    prg_bank1: &*cart.get_bank(0, BankType::PrgRom),
                    prg_bank2: &*cart.get_bank(bank_num - 1, BankType::PrgRom),
                    chr_bank: &mut *cart.get_bank(0, BankType::ChrRom),
                    sram: &mut *cart.get_bank(0, BankType::Sram),
                    bank_num}
        }
    }
}
