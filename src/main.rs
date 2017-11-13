extern crate core;
mod memory;
mod mos6502;
mod ppu;
mod cartridge;
mod mapper;

use std::fs::File;
use std::io::Read;
use core::cell::RefCell;
use core::intrinsics::transmute;
use cartridge::*;


extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Rect;

struct Window {
    buff: RefCell<[[u8; 256]; 240]>
}

impl ppu::Screen for Window {
    fn put(&self, x: u8, y: u8, color: u8) {
        self.buff.borrow_mut()[y as usize][x as usize] = color;
    }
    fn render(&self) {
        println!("a frame has been redrawn:");
        for r in self.buff.borrow().iter() {
            for c in r.iter() {
                print!("{:02x}", c);
            }
            println!("");
        }
    }
}

struct SDLWindow {
    canvas: RefCell<sdl2::render::WindowCanvas>
}

impl SDLWindow {
    fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem.window("rust-sdl2 demo: Video", 256 * PIXEL_SIZE, 240 * PIXEL_SIZE)
                                    .position_centered()
                                    .opengl()
                                    .build()
                                    .unwrap();
        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();
        SDLWindow {
            canvas: RefCell::new(canvas)
        }
    }
}

const PIXEL_SIZE: u32 = 2;
const COLORS: [u32; 64] = [
    0x666666, 0x002A88, 0x1412A7, 0x3B00A4, 0x5C007E, 0x6E0040, 0x6C0600, 0x561D00,
    0x333500, 0x0B4800, 0x005200, 0x004F08, 0x00404D, 0x000000, 0x000000, 0x000000,
    0xADADAD, 0x155FD9, 0x4240FF, 0x7527FE, 0xA01ACC, 0xB71E7B, 0xB53120, 0x994E00,
    0x6B6D00, 0x388700, 0x0C9300, 0x008F32, 0x007C8D, 0x000000, 0x000000, 0x000000,
    0xFFFEFF, 0x64B0FF, 0x9290FF, 0xC676FF, 0xF36AFF, 0xFE6ECC, 0xFE8170, 0xEA9E22,
    0xBCBE00, 0x88D800, 0x5CE430, 0x45E082, 0x48CDDE, 0x4F4F4F, 0x000000, 0x000000,
    0xFFFEFF, 0xC0DFFF, 0xD3D2FF, 0xE8C8FF, 0xFBC2FF, 0xFEC4EA, 0xFECCC5, 0xF7D8A5,
    0xE4E594, 0xCFEF96, 0xBDF4AB, 0xB3F3CC, 0xB5EBF2, 0xB8B8B8, 0x000000, 0x000000,
];

fn get_rgb(color: u8) -> Color {
    let c = COLORS[color as usize];
    Color::RGB((c >> 16) as u8, ((c >> 8) & 0xff) as u8, (c & 0xff) as u8)
}

impl ppu::Screen for SDLWindow {
    fn put(&self, x: u8, y: u8, color: u8) {
        let mut canvas = self.canvas.borrow_mut();
        println!("put {} at {}, {}", color, x, y);
        canvas.set_draw_color(get_rgb(color));
        canvas.draw_rect(Rect::new((x as u32 * PIXEL_SIZE) as i32,
                                   (y as u32 * PIXEL_SIZE) as i32,
                                   PIXEL_SIZE, PIXEL_SIZE));
    }

    fn render(&self) {
        let mut canvas = self.canvas.borrow_mut();
        canvas.present();
        canvas.clear();
    }
}

#[repr(C, packed)]
struct INesHeader {
    magic: [u8; 4],
    prg_rom_nbanks: u8,
    chr_rom_nbanks: u8,
    flags6: u8,
    flags7: u8,
    prg_ram_nbanks: u8,
    flags9: u8,
    flags10: u8,
    padding: [u8; 5]
}

fn main() {
    let fname = std::env::args().nth(1).unwrap();
    let mut file = File::open(fname).unwrap();
    let mut rheader = [0; 16];
    println!("read {}", file.read(&mut rheader[..]).unwrap());
    let header = unsafe{transmute::<[u8; 16], INesHeader>(rheader)};
    let mirror = match ((header.flags6 >> 2) & 2) | (header.flags6 & 1) {
        0 => MirrorType::Horizontal,
        1 => MirrorType::Vertical,
        2 => MirrorType::Single0,
        3 => MirrorType::Single1,
        _ => MirrorType::Four,
    };
    let mapper_id = (header.flags7 & 0xf0) | (header.flags6 >> 4);
    println!("maigc:{} prg:{} chr:{} mirror:{} mapper:{}",
             std::str::from_utf8(&header.magic).unwrap(),
             header.prg_rom_nbanks,
             header.chr_rom_nbanks,
             mirror as u8,
             mapper_id);
    if header.flags6 & 0x04 == 0x04 {
        let mut trainer: [u8; 512] = unsafe{std::mem::uninitialized()};
        file.read(&mut trainer[..]);
        println!("skipping trainer");
    }
    
    let prg_len = header.prg_rom_nbanks as usize * 0x4000;
    let mut chr_len = header.chr_rom_nbanks as usize * 0x2000;
    if chr_len == 0 {
        chr_len = 0x2000;
    }
    let mut prg_rom = Vec::<u8>::with_capacity(prg_len);
    let mut chr_rom = Vec::<u8>::with_capacity(chr_len);
    unsafe {
        prg_rom.set_len(prg_len);
        chr_rom.set_len(chr_len);
    }
    let sram = vec![0; 0x4000];
    println!("read prg {}", file.read(&mut prg_rom[..]).unwrap());
    for (i, v) in prg_rom.iter().enumerate() {
        print!("{:02x} ", v);
        if i & 15 == 15 {
            println!(" {:04x}", i);
        }
    }
    println!("read chr {}", file.read(&mut chr_rom[..]).unwrap());
    for (i, v) in chr_rom.iter().enumerate() {
        print!("{:02x} ", v);
        if i & 15 == 15 {
            println!("");
        }
    }
    let cart = cartridge::Cartridge::new(chr_rom, prg_rom, sram, mirror);
    //let win = Window {buff: RefCell::new([[0; 256]; 240])};
    let win = SDLWindow::new();
    let mapper = mapper::Mapper2::new(&cart);
    let pmem = memory::PPUMemory::new(&mapper, &cart);
    let mem = memory::CPUMemory::new(&mapper);
    let mut ppu = ppu::PPU::new(&pmem, &win);
    let mut cpu = mos6502::CPU::new(&mem);
    mem.init(&mut cpu, &mut ppu);

    loop {
        cpu.step();
        //println!("cpu at 0x{:04x}", cpu.get_pc());
        while cpu.cycle > 0 {
            for _ in 0..3 {
                if ppu.tick() {
                    println!("triggering nmi");
                    cpu.trigger_nmi();
                }
            }
            cpu.cycle -= 1;
        }
    }
}
