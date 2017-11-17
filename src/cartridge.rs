#![allow(dead_code)]

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
    chr_rom: Vec<u8>,
    prg_rom: Vec<u8>,
    sram: Vec<u8>,
    pub mirror_type: MirrorType
}

impl Cartridge {
    pub fn get_size(&self, kind: BankType) -> usize {
        match kind {
            BankType::PrgRom => self.prg_rom.len(),
            BankType::ChrRom => self.chr_rom.len(),
            BankType::Sram => self.sram.len()
        }
    }
    pub fn get_bank(&mut self, base: usize, size: usize, kind: BankType) -> *mut [u8] {
        &mut (match kind {
            BankType::PrgRom => &mut self.prg_rom,
            BankType::ChrRom => &mut self.chr_rom,
            BankType::Sram => &mut self.sram,
        })[base..base + size]
    }
    pub fn new(chr_rom: Vec<u8>,
               prg_rom: Vec<u8>,
               sram: Vec<u8>,
               mirror_type: MirrorType) -> Self {
        Cartridge{chr_rom, prg_rom, sram, mirror_type}
    }
}
