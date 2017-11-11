use memory::VMem;
use cartridge::{Cartridge, BankType};
pub struct Mapper2<'a, T: 'a> where T: Cartridge {
    cart: &'a T,
    prg_bank1: &'a [u8],
    prg_bank2: &'a [u8],
    chr_bank: &'a mut [u8],
    sram: &'a mut [u8],
    bank_num: usize
}

impl<'a, T> VMem for Mapper2<'a, T> where T: Cartridge {
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
                data as usize % self.bank_num, BankType::PRG_ROM)};
        } else if addr >= 0x6000 {
            self.sram[addr - 0x6000] = data;
        } else {
            panic!("invalid write to address: 0x{:04x}", addr);
        }
    }
}

impl<'a, T> Mapper2<'a, T>  where T: Cartridge {
    fn new(cart: &'a mut T) -> Self {
        let bank_num = cart.get_bank_num(BankType::PRG_ROM);
        unsafe {
            Mapper2{cart,
                    prg_bank1: &*cart.get_bank(0, BankType::PRG_ROM),
                    prg_bank2: &*cart.get_bank(bank_num - 1, BankType::PRG_ROM),
                    chr_bank: &mut *cart.get_bank(0, BankType::CHR_ROM),
                    sram: &mut *cart.get_bank(0, BankType::SRAM),
                    bank_num}
        }
    }
}
