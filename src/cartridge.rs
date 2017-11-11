pub enum BankType {
    PRG_ROM, /* program rom */
    CHR_ROM, /* pattern rom */
    SRAM,    /* save ram */
}

pub trait Cartridge {
    fn get_bank_num(&self, kind: BankType) -> usize;
    fn get_bank(&self, idx: usize, kind: BankType) -> *mut [u8];
}
