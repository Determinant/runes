#![allow(dead_code)]
use core::cell::RefCell;

#[derive(Copy, Clone)]
pub enum MirrorType {
    Horizontal = 0,
    Vertical = 1,
    Single0 = 2,
    Single1 = 3,
    Four = 4
}

pub enum BankType {
    PrgRom, /* program rom */
    ChrRom, /* pattern rom */
    Sram,    /* save ram */
}

pub struct Cartridge {
    chr_rom: RefCell<Vec<u8>>,
    prg_rom: RefCell<Vec<u8>>,
    sram: RefCell<Vec<u8>>,
    pub mirror_type: MirrorType
}

impl Cartridge {
    pub fn get_size(&self, kind: BankType) -> usize {
        match kind {
            BankType::PrgRom => self.prg_rom.borrow().len(),
            BankType::ChrRom => self.chr_rom.borrow().len(),
            BankType::Sram => self.sram.borrow().len()
        }
    }
    pub fn get_bank(&self, base: usize, size: usize, kind: BankType) -> *mut [u8] {
        &mut (match kind {
            BankType::PrgRom => self.prg_rom.borrow_mut(),
            BankType::ChrRom => self.chr_rom.borrow_mut(),
            BankType::Sram => self.sram.borrow_mut(),
        })[base..base + size]
    }
    pub fn new(chr_rom: Vec<u8>,
               prg_rom: Vec<u8>,
               sram: Vec<u8>,
               mirror_type: MirrorType) -> Self {
        Cartridge{chr_rom: RefCell::new(chr_rom),
                  prg_rom: RefCell::new(prg_rom),
                  sram: RefCell::new(sram), mirror_type}
    }
}
