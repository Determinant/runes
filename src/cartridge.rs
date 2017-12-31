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

pub trait Cartridge {
    fn get_size(&self, kind: BankType) -> usize;
    fn get_bank<'a>(&mut self, base: usize, size: usize, kind: BankType) -> &'a mut [u8];
    #[inline(always)] fn get_mirror_type(&self) -> MirrorType;
    #[inline(always)] fn set_mirror_type(&mut self, mt: MirrorType);
}
