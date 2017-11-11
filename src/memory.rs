pub trait VMem {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

pub struct CPUMemory {
    internal: [u8; 2048]
}

impl CPUMemory {
    pub fn new() -> Self {
        CPUMemory{internal: [0; 2048]}
    }
}

impl VMem for CPUMemory {
    fn read(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            self.internal[(addr & 0x07ff) as usize]
        } else if addr < 0x4000 {
            match addr & 0x7 {
                _ => 0
            }
        } else {
            panic!("invalid memory read access at 0x{:04x}", addr)
        }
    }
    fn write(&mut self, addr: u16, data: u8) {
        if addr < 0x2000 {
            self.internal[(addr & 0x07ff) as usize] = data;
        } else if addr < 0x4000 {
            match addr & 0x7 {
                _ => ()
            }
        } else {
            panic!("invalid memory write access at 0x{:04x}", addr)
        }
    }
}
