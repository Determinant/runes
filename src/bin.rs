extern crate core;

use std::fs::File;
use std::sync::{Mutex, Condvar};
use std::io::Read;
use std::cell::RefCell;
use std::intrinsics::transmute;
//use std::time::{Instant, Duration};
//use std::thread;

extern crate sdl2;
#[macro_use] extern crate clap;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use clap::{Arg, App};

mod memory;
#[macro_use] mod mos6502;
mod ppu;
mod apu;
mod cartridge;
mod mapper;
mod controller;
mod disasm;

use mos6502::CPU;
use ppu::PPU;
use apu::APU;
use memory::{CPUMemory, PPUMemory};
use cartridge::{BankType, MirrorType, Cartridge};
use controller::stdctl;

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

const PIX_WIDTH: u32 = 256;
const PIX_HEIGHT: u32 = 240;
const FB_PITCH: usize = PIX_WIDTH as usize * 3;
const FB_SIZE: usize = PIX_HEIGHT as usize * FB_PITCH;
const AUDIO_SAMPLES: u16 = 4410;
const AUDIO_EXTRA_SAMPLES: u16 = 20;
const AUDIO_ALL_SAMPLES: u16 = AUDIO_SAMPLES + AUDIO_EXTRA_SAMPLES;

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
    p1_button_state: u8,
    p1_ctl: &'a stdctl::Joystick,
    p1_keymap: [u8; 256],
}

macro_rules! gen_keymap {
    ($tab: ident, [$($x: expr, $y: expr), *]) => {
        {$($tab[($x as usize) & 0xff] = $y;)*}
    };
}

impl<'a> SDLWindow<'a> {
    fn new(sdl_context: &'a sdl2::Sdl,
           p1_ctl: &'a stdctl::Joystick,
           pixel_scale: u32) -> Self {
        use Keycode::*;
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem.window("RuNES", PIX_WIDTH * pixel_scale,
                                                    PIX_HEIGHT * pixel_scale)
                                    .position_centered()
                                    .opengl()
                                    .build()
                                    .unwrap();
        let mut canvas = window.into_canvas()
                                    .accelerated()
                                    .present_vsync()
                                    .build().unwrap();
        let texture_creator = canvas.texture_creator();
        canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();
        let mut res = SDLWindow {
            canvas,
            events: sdl_context.event_pump().unwrap(),
            frame_buffer: [0; FB_SIZE],
            texture: texture_creator.create_texture_streaming(
                        sdl2::pixels::PixelFormatEnum::RGB24,
                        PIX_WIDTH, PIX_HEIGHT).unwrap(),
            p1_button_state: 0,
            p1_ctl, p1_keymap: [stdctl::NULL; 256],
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

#[inline(always)]
fn get_rgb(color: u8) -> (u8, u8, u8) {
    let c = RGB_COLORS[color as usize];
    ((c >> 16) as u8, ((c >> 8) & 0xff) as u8, (c & 0xff) as u8)
}


impl<'a> ppu::Screen for SDLWindow<'a> {
    #[inline(always)]
    fn put(&mut self, x: u8, y: u8, color: u8) {
        let (r, g, b) = get_rgb(color);
        let base = (y as usize * FB_PITCH) + x as usize * 3;
        self.frame_buffer[base] = r;
        self.frame_buffer[base + 1] = g;
        self.frame_buffer[base + 2] = b;
    }

    fn render(&mut self) {
        self.texture.update(None, &self.frame_buffer, FB_PITCH).unwrap();
    }

    fn frame(&mut self) {
        self.canvas.clear();
        self.canvas.copy(&self.texture, None, None).unwrap();
        self.canvas.present();
        if self.poll() {std::process::exit(0);}
    }
}

struct CircularBuffer {
    buffer: [i16; 2 * AUDIO_ALL_SAMPLES as usize],
    head: usize,
    tail: usize
}

impl CircularBuffer {
    fn new() -> Self {
        CircularBuffer {
            buffer: [0; 2 * AUDIO_ALL_SAMPLES as usize],
            head: 0,
            tail: AUDIO_ALL_SAMPLES as usize
        }
    }

    fn enque(&mut self, sample: i16) {
        self.buffer[self.tail] = sample;
        self.tail += 1;
        if self.tail == self.buffer.len() {
            self.tail = 0
        }
    }

    fn deque(&mut self) -> i16 {
        let res = self.buffer[self.head];
        if self.head != self.tail {
            let mut h = self.head + 1;
            if h == self.buffer.len() {
                h = 0
            }
            if h != self.tail {
                self.head = h
            } else {
                self.tail = self.head
            }
        }
        res
    }
}

struct AudioSync {
    time_barrier: Condvar,
    buffer: Mutex<(CircularBuffer, u16)>,
}

struct SDLAudio<'a>(&'a AudioSync);
struct SDLAudioPlayback<'a>(&'a AudioSync);

impl<'a> sdl2::audio::AudioCallback for SDLAudioPlayback<'a> {
    type Channel = i16;
    fn callback(&mut self, out: &mut[i16]) {
        let mut m = self.0.buffer.lock().unwrap();
        {
            let b = &mut m.0;
            let l1 = (b.tail + b.buffer.len() - b.head) % b.buffer.len();
            print!("{} ", l1);
            
            for x in out.iter_mut() {
                *x = b.deque()
            }
        }
        println!("{}", m.1);
        if m.1 >= AUDIO_SAMPLES {
            m.1 -= AUDIO_SAMPLES;
            self.0.time_barrier.notify_one();
        } else {
            m.1 = 0;
            println!("audio frame skipping");
        }
    }
}

impl<'a> apu::Speaker for SDLAudio<'a> {
    fn queue(&mut self, sample: u16) {
        let mut m = self.0.buffer.lock().unwrap();
        {
            let b = &mut m.0;
            b.enque(sample.wrapping_sub(1 << 15) as i16);
        }
        m.1 += 1;
        while m.1 >= AUDIO_ALL_SAMPLES {
            m = self.0.time_barrier.wait(m).unwrap();
        }
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

fn print_cpu_trace(cpu: &CPU) {
    use disasm;
    let pc = cpu.get_pc();
    let mem = cpu.get_mem();
    let opcode = mem.read_without_tick(pc) as usize;
    let len = mos6502::INST_LENGTH[opcode];
    let mut code = vec![0; len as usize];
    for i in 0..len as u16 {
        code[i as usize] = mem.read_without_tick(pc + i);
    }
    println!("0x{:04x} {} a:{:02x} x:{:02x} y:{:02x} s: {:02x} sp: {:02x}",
             pc, disasm::parse(opcode as u8, &code[1..]),
             cpu.get_a(), cpu.get_x(), cpu.get_y(), cpu.get_status(), cpu.get_sp());
}

fn main() {
    let matches = App::new("RuNES")
                    .version("0.1.2")
                    .author("Ted Yin <tederminant@gmail.com>")
                    .about("A Rust NES emulator")
                    .arg(Arg::with_name("scale")
                         .short("x")
                         .long("scale")
                         .required(false)
                         .takes_value(true))
                    .arg(Arg::with_name("INPUT")
                         .help("the iNES ROM file")
                         .required(true)
                         .index(1))
                    .get_matches();
    let scale = std::cmp::min(8,
                    std::cmp::max(1,
                        value_t!(matches, "scale", u32).unwrap_or(4)));
    let fname = matches.value_of("INPUT").unwrap();
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
    println!("magic:{}, prg size:{}, chr size:{}, mirror type:{}, mapper:{}",
             std::str::from_utf8(&header.magic).unwrap(),
             header.prg_rom_nbanks,
             header.chr_rom_nbanks,
             mirror as u8,
             mapper_id);
    if header.flags6 & 0x04 == 0x04 {
        let mut trainer: [u8; 512] = [0; 512];
        file.read(&mut trainer[..]).unwrap();
        println!("skipping trainer");
    }
    
    let prg_len = header.prg_rom_nbanks as usize * 0x4000;
    let mut chr_len = header.chr_rom_nbanks as usize * 0x2000;
    if chr_len == 0 {
        chr_len = 0x2000;
    }
    let mut prg_rom = vec![0; prg_len];
    let mut chr_rom = vec![0; chr_len];
    let sram = vec![0; 0x4000];
    println!("read prg {}", file.read(&mut prg_rom[..]).unwrap());
    println!("read chr {}", file.read(&mut chr_rom[..]).unwrap());

    /* audio */
    let sdl_context = sdl2::init().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();
    let audio_sync = AudioSync { time_barrier: Condvar::new(),
                                 buffer: Mutex::new((CircularBuffer::new(),
                                                     AUDIO_ALL_SAMPLES))};
    let mut spkr = SDLAudio(&audio_sync);
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(apu::AUDIO_SAMPLE_FREQ as i32),
        channels: Some(1),
        samples: Some(AUDIO_SAMPLES)
    };
    let device = audio_subsystem.open_playback(None, &desired_spec, |_| {
        SDLAudioPlayback(&audio_sync)
    }).unwrap();

    let p1ctl = stdctl::Joystick::new();
    let cart = SimpleCart::new(chr_rom, prg_rom, sram, mirror);
    let mut win = Box::new(SDLWindow::new(&sdl_context, &p1ctl, scale));
    let mut m: Box<mapper::Mapper> = match mapper_id {
        0 | 2 => Box::new(mapper::Mapper2::new(cart)),
        1 => Box::new(mapper::Mapper1::new(cart)),
        _ => panic!("unsupported mapper {}", mapper_id)
    };

    let mapper = RefCell::new(&mut (*m) as &mut mapper::Mapper);
    let mut cpu = CPU::new(CPUMemory::new(&mapper, Some(&p1ctl), None)/*, &mut f*/);
    let mut ppu = PPU::new(PPUMemory::new(&mapper), &mut (*win));
    let mut apu = APU::new(&mut spkr);
    let cpu_ptr = &mut cpu as *mut CPU;
    cpu.mem.bus.attach(cpu_ptr, &mut ppu, &mut apu);
    cpu.powerup();
    device.resume();
    loop {
        /* consume the leftover cycles from the last instruction */
        while cpu.cycle > 0 {
            cpu.mem.bus.tick()
        }
        //print_cpu_trace(&cpu);
        cpu.step();
    }
}
