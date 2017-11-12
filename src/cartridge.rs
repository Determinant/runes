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
    chr_rom: [u8; 8192],
    prg_rom: RefCell<[u8; 8192]>,
    sram: [u8; 8192],
    pub mirror_type: MirrorType
}

impl Cartridge {
    pub fn get_bank_num(&self, kind: BankType) -> usize {0}
    pub fn get_bank(&self, idx: usize, kind: BankType) -> *mut [u8] {
        &mut *self.prg_rom.borrow_mut()
    }
    pub fn new() -> Self {
        Cartridge {
            chr_rom: [0; 8192],
            prg_rom: RefCell::new([0; 8192]),
            sram: [0; 8192],
            mirror_type: MirrorType::Horizontal
        }
    }
}
