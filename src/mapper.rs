#![allow(dead_code)]
use memory::VMem;
use cartridge::{Cartridge, BankType};
use core::cell::RefCell;

pub struct Mapper2<'a> {
    cart: &'a Cartridge,
    prg_bank1: RefCell<&'a [u8]>,
    prg_bank2: RefCell<&'a [u8]>,
    chr_bank: RefCell<&'a mut [u8]>,
    sram: RefCell<&'a mut [u8]>,
    prg_nbank: usize
}

impl<'a> VMem for Mapper2<'a> {
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < 0x2000 {         /* 0x2000 size bank */
            self.chr_bank.borrow()[addr]
        } else if addr >= 0xc000 { /* 0x4000 size bank */
            self.prg_bank2.borrow()[addr - 0xc000]
        } else if addr >= 0x8000 { /* 0x4000 size bank */
            self.prg_bank1.borrow()[addr - 0x8000]
        } else if addr >= 0x6000 {
            self.sram.borrow()[addr - 0x6000]
        } else {
            panic!("unmapped address: 0x{:04x}", addr)
        }
    }

    fn write(&self, addr: u16, data: u8) {
        let addr = addr as usize;
        if addr < 0x2000 {
            self.chr_bank.borrow_mut()[addr] = data;
        } else if addr >= 0x8000 {
            (*self.prg_bank1.borrow_mut()) = unsafe {
                &*self.cart.get_bank(((data as usize) % self.prg_nbank) << 14,
                                    0x4000,
                                    BankType::PrgRom)};
        } else if addr >= 0x6000 {
            self.sram.borrow_mut()[addr - 0x6000] = data;
        } else {
            panic!("invalid write to address: 0x{:04x}", addr);
        }
    }
}

impl<'a> Mapper2<'a> {
    pub fn new(cart: *const Cartridge) -> Self {
        unsafe {
            let cart = &*cart;
            let nbank = cart.get_size(BankType::PrgRom) >> 14;
            Mapper2{cart: &cart,
            prg_bank1: RefCell::new(&*cart.get_bank(0, 0x4000, BankType::PrgRom)),
            prg_bank2: RefCell::new(&*cart.get_bank((nbank - 1) << 14, 0x4000, BankType::PrgRom)),
            chr_bank: RefCell::new(&mut *cart.get_bank(0, 0x2000, BankType::ChrRom)),
            sram: RefCell::new(&mut *cart.get_bank(0, 0x2000, BankType::Sram)),
            prg_nbank: nbank}
        }
    }
}
