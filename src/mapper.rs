#![allow(dead_code)]
use memory::VMem;
use cartridge::{Cartridge, BankType};
use core::cell::UnsafeCell;

pub struct Mapper2<'a> {
    cart: &'a Cartridge,
    prg_bank1: UnsafeCell<&'a [u8]>,
    prg_bank2: UnsafeCell<&'a [u8]>,
    chr_bank: UnsafeCell<&'a mut [u8]>,
    sram: UnsafeCell<&'a mut [u8]>,
    prg_nbank: usize
}

impl<'a> VMem for Mapper2<'a> {
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        unsafe {
            if addr < 0x2000 {         /* 0x2000 size bank */
                (*self.chr_bank.get())[addr]
            } else if addr >= 0xc000 { /* 0x4000 size bank */
                (*self.prg_bank2.get())[addr - 0xc000]
            } else if addr >= 0x8000 { /* 0x4000 size bank */
                (*self.prg_bank1.get())[addr - 0x8000]
            } else if addr >= 0x6000 {
                (*self.sram.get())[addr - 0x6000]
            } else {
                panic!("unmapped address: 0x{:04x}", addr)
            }
        }
    }

    fn write(&self, addr: u16, data: u8) {
        let addr = addr as usize;
        unsafe {
            if addr < 0x2000 {
                (*self.chr_bank.get())[addr] = data;
            } else if addr >= 0x8000 {
                (*self.prg_bank1.get()) =
                    &*self.cart.get_bank(((data as usize) % self.prg_nbank) << 14,
                    0x4000,
                    BankType::PrgRom);
            } else if addr >= 0x6000 {
                (*self.sram.get())[addr - 0x6000] = data;
            } else {
                panic!("invalid write to address: 0x{:04x}", addr);
            }
        }
    }
}

impl<'a> Mapper2<'a> {
    pub fn new(cart: &'a Cartridge) -> Self {
        unsafe {
            let nbank = cart.get_size(BankType::PrgRom) >> 14;
            Mapper2{cart,
                prg_bank1: UnsafeCell::new(&*cart.get_bank(0, 0x4000, BankType::PrgRom)),
                prg_bank2: UnsafeCell::new(&*cart.get_bank((nbank - 1) << 14, 0x4000, BankType::PrgRom)),
                chr_bank: UnsafeCell::new(&mut *cart.get_bank(0, 0x2000, BankType::ChrRom)),
                sram: UnsafeCell::new(&mut *cart.get_bank(0, 0x2000, BankType::Sram)),
                prg_nbank: nbank}
        }
    }
}
