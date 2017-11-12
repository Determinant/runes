extern crate core;
mod memory;
mod mos6502;
mod ppu;
mod cartridge;
mod mapper;
use core::ptr::null_mut;

struct Window {
}

impl ppu::Screen for Window {
    fn put(&mut self, x: u8, y: u8, color: u8) {
        println!("put color 0x{:02x} at ({}, {})", color, x, y);
    }
    fn render(&self) {
        println!("a frame has been redrawn");
    }
}

fn main() {
    //let mut ppu = ppu::PPU::new(
    let mut cart = cartridge::Cartridge::new();
    let mut mapper = mapper::Mapper2::new(&mut cart);
    let mut mem = memory::CPUMemory::new(null_mut(), null_mut(), &mut mapper);
    let cpu = mos6502::CPU::new(&mut mem);
}
