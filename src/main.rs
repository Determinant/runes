extern crate core;
mod memory;
mod mos6502;
mod ppu;
mod cartridge;
mod mapper;

use std::fs::File;
use std::io::Read;
use core::cell::{RefCell, UnsafeCell};
use core::intrinsics::transmute;
use cartridge::*;
use std::time::{Instant, Duration};
use std::thread::sleep;

extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

struct DummyWindow {
    buff: RefCell<[[u8; 240]; 256]>
}

impl ppu::Screen for DummyWindow {
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
    canvas: RefCell<sdl2::render::WindowCanvas>,
    events: RefCell<sdl2::EventPump>,
    timer: RefCell<sdl2::TimerSubsystem>,
    frame_buffer: UnsafeCell<[[u8; 240]; 256]>,
}

impl SDLWindow {
    fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let timer = sdl_context.timer().unwrap();
        let window = video_subsystem.window("RuNES", 256 * PIXEL_SIZE, 240 * PIXEL_SIZE)
                                    .position_centered()
                                    .opengl()
                                    .build()
                                    .unwrap();
        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();
        SDLWindow {
            canvas: RefCell::new(canvas),
            timer: RefCell::new(timer),
            events: RefCell::new(sdl_context.event_pump().unwrap()),
            frame_buffer: UnsafeCell::new([[0; 240]; 256])
        }
    }

    fn poll(&self) -> bool {
        for event in self.events.borrow_mut().poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    return true;
                },
                _ => ()
            }
        }
        false
    }
}

const PIXEL_SIZE: u32 = 2;
const RGB_COLORS: [u32; 64] = [
    0x666666, 0x002a88, 0x1412a7, 0x3b00a4, 0x5c007e, 0x6e0040, 0x6c0600, 0x561d00,
    0x333500, 0x0b4800, 0x005200, 0x004f08, 0x00404d, 0x000000, 0x000000, 0x000000,
    0xadadad, 0x155fd9, 0x4240ff, 0x7527fe, 0xa01acc, 0xb71e7b, 0xb53120, 0x994e00,
    0x6b6d00, 0x388700, 0x0c9300, 0x008f32, 0x007c8d, 0x000000, 0x000000, 0x000000,
    0xfffeff, 0x64b0ff, 0x9290ff, 0xc676ff, 0xf36aff, 0xfe6ecc, 0xfe8170, 0xea9e22,
    0xbcbe00, 0x88d800, 0x5ce430, 0x45e082, 0x48cdde, 0x4f4f4f, 0x000000, 0x000000,
    0xfffeff, 0xc0dfff, 0xd3d2ff, 0xe8c8ff, 0xfbc2ff, 0xfec4ea, 0xfeccc5, 0xf7d8a5,
    0xe4e594, 0xcfef96, 0xbdf4ab, 0xb3f3cc, 0xb5ebf2, 0xb8b8b8, 0x000000, 0x000000,
];

fn get_rgb(color: u8) -> Color {
    let c = RGB_COLORS[color as usize];
    Color::RGB((c >> 16) as u8, ((c >> 8) & 0xff) as u8, (c & 0xff) as u8)
}

impl ppu::Screen for SDLWindow {
    fn put(&self, x: u8, y: u8, color: u8) {
        unsafe {
            (*self.frame_buffer.get())[x as usize][y as usize] = color;
        }
    }

    fn render(&self) {
        let mut canvas = self.canvas.borrow_mut();
        let fb = unsafe{&*self.frame_buffer.get()};
        for (x, l) in fb.iter().enumerate() {
            for (y, c) in l.iter().enumerate() {
                canvas.set_draw_color(get_rgb(*c));
                canvas.fill_rect(Rect::new((x as u32 * PIXEL_SIZE) as i32,
                                            (y as u32 * PIXEL_SIZE) as i32,
                                            PIXEL_SIZE, PIXEL_SIZE));
            }
        }
        canvas.present();
        canvas.set_draw_color(Color::RGB(128, 128, 128));
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
    /*
    for (i, v) in prg_rom.iter().enumerate() {
        print!("{:02x} ", v);
        if i & 15 == 15 {
            println!(" {:04x}", i);
        }
    }
    */
    println!("read chr {}", file.read(&mut chr_rom[..]).unwrap());
    /*
    for (i, v) in chr_rom.iter().enumerate() {
        print!("{:02x} ", v);
        if i & 15 == 15 {
            println!("");
        }
    }
    */
    let cart = cartridge::Cartridge::new(chr_rom, prg_rom, sram, mirror);
    //let win = Window {buff: RefCell::new([[0; 256]; 240])};
    let win = SDLWindow::new();
    let mapper = mapper::Mapper2::new(&cart);
    let pmem = memory::PPUMemory::new(&mapper, &cart);
    let mem = memory::CPUMemory::new(&mapper);
    let mut ppu = ppu::PPU::new(&pmem, &win);
    let mut cpu = mos6502::CPU::new(&mem);
    mem.init(&mut cpu, &mut ppu);
    let mut cnt = 0;
    use ppu::Screen;
    const CYC_PER_FRAME: u32 = mos6502::CPU_FREQ / 60;
    let duration_per_frame: Duration = Duration::from_millis(1000 / 60);
    let mut timer = Instant::now();
    loop {
        if cnt >= CYC_PER_FRAME {
            win.render();
            if win.poll() {break}
            let e = timer.elapsed();
            if duration_per_frame > e {
                sleep(duration_per_frame - e);
                println!("faster {}", (duration_per_frame - e).subsec_nanos() as f64 / 1e6);
            } else {
                println!("slower");
            }
            timer = Instant::now();
            cnt -= CYC_PER_FRAME;
        }
        cpu.step();
        //println!("cpu at 0x{:04x}", cpu.get_pc());
        while cpu.cycle > 0 {
            for _ in 0..3 {
                if ppu.tick() {
                    cpu.trigger_nmi();
                }
            }
            cnt += 1;
            cpu.cycle -= 1;
        }
    }
}
