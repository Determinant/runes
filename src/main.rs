extern crate core;
mod memory;
mod mos6502;
mod ppu;
mod cartridge;
mod mapper;
fn main() {
    /*
    let code = [0xa9, 0x01, 0x8d, 0x00, 0x02, 0xa9, 0x05, 0x8d, 0x01, 0x02, 0xa9, 0x08, 0x8d, 0x02, 0x02 ];
    let code2 = [0xa9, 0x03, 0x4c, 0x08, 0x06, 0x00, 0x00, 0x00, 0x8d, 0x00, 0x02 ];
    let dasm = mos6502::disasm::Disassembler::new(code2.iter());
    for l in dasm {
        println!("{}", l);
    }
    let a = 0x03;
    let b = 0x4c;
    let c = 0x08;
    let d = 0x06;
    println!("{}", disasm::parse(code2[0], &[a, b, c, d]));
    */
    let mut mem = memory::CPUMemory::new();
    let cpu = mos6502::CPU::new(&mut mem);
}
