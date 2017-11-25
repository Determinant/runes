extern crate core;

use std::fs::File;
use std::io::Read;
use std::cell::RefCell;
use std::intrinsics::transmute;
use std::time::{Instant, Duration};
use std::thread::sleep;

extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

mod memory;
#[macro_use] mod mos6502;
mod ppu;
mod cartridge;
mod mapper;
mod controller;
mod disasm;

use mos6502::CPU;
use ppu::PPU;
use memory::{CPUMemory, PPUMemory};
use cartridge::{BankType, MirrorType, Cartridge};
use controller::stdctl;

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

const PIX_WIDTH: usize = 256;
const PIX_HEIGHT: usize = 240;
const FB_PITCH: usize = PIX_WIDTH * 3 * (PIXEL_SIZE as usize);
const FB_SIZE: usize = PIX_HEIGHT * FB_PITCH * (PIXEL_SIZE as usize);
const WIN_WIDTH: u32 = PIX_WIDTH as u32 * PIXEL_SIZE;
const WIN_HEIGHT: u32 = PIX_HEIGHT as u32 * PIXEL_SIZE;

pub struct SimpleCart {
    chr_rom: Vec<u8>,
    prg_rom: Vec<u8>,
    sram: Vec<u8>,
    pub mirror_type: MirrorType
}

impl SimpleCart {
    pub fn new(chr_rom: Vec<u8>,
               prg_rom: Vec<u8>,
               sram: Vec<u8>,
               mirror_type: MirrorType) -> Self {
        SimpleCart{chr_rom, prg_rom, sram, mirror_type}
    }
}

impl Cartridge for SimpleCart {
    fn get_size(&self, kind: BankType) -> usize {
        match kind {
            BankType::PrgRom => self.prg_rom.len(),
            BankType::ChrRom => self.chr_rom.len(),
            BankType::Sram => self.sram.len()
        }
    }
    fn get_bank(&mut self, base: usize, size: usize, kind: BankType) -> *mut [u8] {
        &mut (match kind {
            BankType::PrgRom => &mut self.prg_rom,
            BankType::ChrRom => &mut self.chr_rom,
            BankType::Sram => &mut self.sram,
        })[base..base + size]
    }
    fn get_mirror_type(&self) -> MirrorType {self.mirror_type}
    fn set_mirror_type(&mut self, mt: MirrorType) {self.mirror_type = mt}
}

struct SDLWindow<'a> {
    canvas: sdl2::render::WindowCanvas,
    events: sdl2::EventPump,
    frame_buffer: [u8; FB_SIZE],
    texture: sdl2::render::Texture,
    timer: Instant,
    duration_per_frame: Duration,
    p1_button_state: u8,
    p1_ctl: &'a stdctl::Joystick,
    p1_keymap: [u8; 256],
}

macro_rules! gen_keymap {
    ($tab: ident, [$($x: expr, $y: expr), *]) => {
        {
            $(
                $tab[($x as usize) & 0xff] = $y;
            )*
        }
    };
}

impl<'a> SDLWindow<'a> {
    fn new(p1_ctl: &'a stdctl::Joystick) -> Self {
        use Keycode::*;
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem.window("RuNES", WIN_WIDTH, WIN_HEIGHT)
                                    .position_centered()
                                    .opengl()
                                    .build()
                                    .unwrap();
        let mut canvas = window.into_canvas()
                                    .accelerated()
                                    .build().unwrap();
        let texture_creator = canvas.texture_creator();
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();
        let mut res = SDLWindow {
            canvas,
            events: sdl_context.event_pump().unwrap(),
            frame_buffer: [0; FB_SIZE],
            texture: texture_creator.create_texture_streaming(
                        PixelFormatEnum::RGB24, WIN_WIDTH, WIN_HEIGHT).unwrap(),
            timer: Instant::now(),
            duration_per_frame: Duration::from_millis(1000 / 60),
            p1_button_state: 0,
            p1_ctl, p1_keymap: [stdctl::NULL; 256]
        };
        {
            let keymap = &mut res.p1_keymap;
            gen_keymap!(keymap, [I, stdctl::UP,
                                 K, stdctl::DOWN,
                                 J, stdctl::LEFT,
                                 L, stdctl::RIGHT,
                                 Z, stdctl::A,
                                 X, stdctl::B,
                                 Return, stdctl::START,
                                 S, stdctl::SELECT,
                                 Up, stdctl::UP,
                                 Down, stdctl::DOWN,
                                 Left, stdctl::LEFT,
                                 Right, stdctl::RIGHT
                                 ]);
        }
        res
    }

    #[inline]
    fn poll(&mut self) -> bool {
        use Keycode::*;
        let p1_keymap = &self.p1_keymap;
        for event in self.events.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Escape), .. } => {
                    return true;
                },
                Event::KeyDown { keycode: Some(c), .. } => {
                    self.p1_button_state |= p1_keymap[(c as usize) & 0xff];
                    self.p1_ctl.set(self.p1_button_state)
                },
                Event::KeyUp { keycode: Some(c), .. } => {
                    self.p1_button_state &= !p1_keymap[(c as usize) & 0xff];
                    self.p1_ctl.set(self.p1_button_state)
                },
                _ => ()
            }
        }
        false
    }
}

#[inline]
fn get_rgb(color: u8) -> (u8, u8, u8) {
    let c = RGB_COLORS[color as usize];
    ((c >> 16) as u8, ((c >> 8) & 0xff) as u8, (c & 0xff) as u8)
}

impl<'a> ppu::Screen for SDLWindow<'a> {
    fn put(&mut self, x: u8, y: u8, color: u8) {
        let (r, g, b) = get_rgb(color);
        let mut pattern = [0; 3 * PIXEL_SIZE as usize];
        let mut base = ((y as u32 * PIXEL_SIZE) as usize * FB_PITCH) +
            (x as u32 * 3 * PIXEL_SIZE) as usize;
        {
            let mut i = 0;
            for _ in 0..PIXEL_SIZE {
                pattern[i] = r;
                pattern[i + 1] = g;
                pattern[i + 2] = b;
                i += 3;
            }
        }
        for _ in 0..PIXEL_SIZE {
            self.frame_buffer[base..base + 3 * PIXEL_SIZE as usize]
                .copy_from_slice(&pattern[..]);
            base += FB_PITCH;
        }
    }

    fn render(&mut self) {
        self.texture.update(None, &self.frame_buffer, FB_PITCH).unwrap();
        self.canvas.clear();
        self.canvas.copy(&self.texture, None,
                         Some(Rect::new(0, 0, WIN_WIDTH, WIN_HEIGHT))).unwrap();
        self.canvas.present();
        if self.poll() {std::process::exit(0);}
        let e = self.timer.elapsed();
        if self.duration_per_frame > e {
            let diff = self.duration_per_frame - e;
            sleep(diff);
            //println!("{} faster", diff.subsec_nanos() as f64 / 1e6);
        } else {
            //println!("{} slower", (e - duration_per_frame).subsec_nanos() as f64 / 1e6);
        }
        self.timer = Instant::now();
        //canvas.set_draw_color(Color::RGB(128, 128, 128));
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
        file.read(&mut trainer[..]).unwrap();
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
    let p1ctl = stdctl::Joystick::new();
    let cart = SimpleCart::new(chr_rom, prg_rom, sram, mirror);
    let mut win = SDLWindow::new(&p1ctl);
    let mut m: Box<mapper::Mapper> = match mapper_id {
        0 | 2 => Box::new(mapper::Mapper2::new(cart)),
        1 => Box::new(mapper::Mapper1::new(cart)),
        _ => panic!("unsupported mapper {}", mapper_id)
    };
    let mapper = RefCell::new(&mut (*m) as &mut mapper::Mapper);
    let mut cpu = CPU::new(CPUMemory::new(&mapper, Some(&p1ctl), None));
    let mut ppu = PPU::new(PPUMemory::new(&mapper), &mut win);
    let cpu_ptr = &mut cpu as *mut CPU;
    cpu.mem.bus.attach(cpu_ptr, &mut ppu);
    cpu.start();
    loop {
        while cpu.cycle > 0 {
            cpu.mem.ppu_tick()
        }
        /*
        {
            use disasm;
            let pc = cpu.get_pc();
            let mem = cpu.get_mem();
            let opcode = mem.read(pc) as usize;
            let len = mos6502::INST_LENGTH[opcode];
            let mut code = vec![0; len as usize];
            for i in 0..len as u16 {
                code[i as usize] = mem.read(pc + i);
            }
            println!("0x{:04x} {} a:{:02x} x:{:02x} y:{:02x} s: {:02x} sp: {:02x}",
                     pc, disasm::parse(opcode as u8, &code[1..]),
                     cpu.get_a(), cpu.get_x(), cpu.get_y(), cpu.get_status(), cpu.get_sp());
        }
        */
        cpu.step();
    }
}
