use crate::utils::{Read, Write};

#[derive(Copy, Clone)]
pub enum MirrorType {
    Horizontal = 0,
    Vertical = 1,
    Single0 = 2,
    Single1 = 3,
    Four = 4,
}

pub enum BankType {
    PrgRom, /* program rom */
    ChrRom, /* pattern rom */
    Sram,   /* save ram */
}

pub trait Cartridge {
    fn get_size(&self, kind: BankType) -> usize;
    fn get_bank<'a>(
        &self,
        base: usize,
        size: usize,
        kind: BankType,
    ) -> &'a [u8];
    fn get_bank_mut<'a>(
        &mut self,
        base: usize,
        size: usize,
        kind: BankType,
    ) -> &'a mut [u8];
    fn get_mirror_type(&self) -> MirrorType;
    fn set_mirror_type(&mut self, mt: MirrorType);
    fn load(&mut self, reader: &mut dyn Read) -> bool;
    fn save(&self, writer: &mut dyn Write) -> bool;
    fn load_sram(&mut self, reader: &mut dyn Read) -> bool;
    fn save_sram(&self, writer: &mut dyn Write) -> bool;
}
