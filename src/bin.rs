extern crate core;

use std::fs::File;
use std::sync::{Mutex, Condvar};
use std::io::{Read, Write};
use std::mem::transmute;
use std::process::exit;
use std::cell::{Cell, RefCell};

extern crate sdl2;
#[macro_use] extern crate clap;

use clap::{Arg, App};

mod utils;
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
use controller::{InputPoller, stdctl};

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
const AUDIO_SAMPLES: u16 = 441;
const AUDIO_EXTRA_SAMPLES: u16 = 4410;
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

    fn load_vec(vec: &mut Vec<u8>, reader: &mut utils::Read) -> bool {
        let len = vec.len();
        match reader.read(vec) {
            Some(x) => x == len,
            None => false
        }
    }

    fn save_vec(vec: &Vec<u8>, writer: &mut utils::Write) -> bool {
        let len = vec.len();
        match writer.write(vec) {
            Some(x) => x == len,
            None => false
        }
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
    fn get_bank<'a>(&self, base: usize, size: usize, kind: BankType) -> &'a [u8] {
        unsafe {
            &*((&(match kind {
                BankType::PrgRom => &self.prg_rom,
                BankType::ChrRom => &self.chr_rom,
                BankType::Sram => &self.sram,
            })[base..base + size]) as *const [u8])
        }
    }

    fn get_bank_mut<'a>(&mut self, base: usize, size: usize, kind: BankType) -> &'a mut [u8] {
        unsafe {
            &mut *((&mut (match kind {
                BankType::PrgRom => &mut self.prg_rom,
                BankType::ChrRom => &mut self.chr_rom,
                BankType::Sram => &mut self.sram,
            })[base..base + size]) as *mut [u8])
        }
    }

    fn get_mirror_type(&self) -> MirrorType {self.mirror_type}
    fn set_mirror_type(&mut self, mt: MirrorType) {self.mirror_type = mt}

    fn load(&mut self, reader: &mut utils::Read) -> bool {
        self.load_sram(reader) &&
        SimpleCart::load_vec(&mut self.chr_rom, reader) &&
        utils::load_prefix(&mut self.mirror_type, 0, reader)
    }

    fn save(&self, writer: &mut utils::Write) -> bool {
        self.save_sram(writer) &&
        SimpleCart::save_vec(&self.chr_rom, writer) &&
        utils::save_prefix(&self.mirror_type, 0, writer)
    }

    fn load_sram(&mut self, reader: &mut utils::Read) -> bool {
        SimpleCart::load_vec(&mut self.sram, reader)
    }

    fn save_sram(&self, writer: &mut utils::Write) -> bool {
        SimpleCart::save_vec(&self.sram, writer)
    }
}

struct FileIO(File);

impl utils::Read for FileIO {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        match self.0.read(buf) {
            Ok(x) => Some(x),
            Err(_) => None
        }
    }
}

impl utils::Write for FileIO {
    fn write(&mut self, buf: &[u8]) -> Option<usize> {
        match self.0.write(buf) {
            Ok(x) => Some(x),
            Err(_) => None
        }
    }
}

struct SDLEventPoller {
    events: RefCell<sdl2::EventPump>,
    p1_button_state: Cell<u8>,
    exit_flag: Cell<bool>,
}

fn keyboard_mapping(code: sdl2::keyboard::Keycode) -> u8 {
    use sdl2::keyboard::Keycode::*;
    match code {
        I => stdctl::UP,
        K => stdctl::DOWN,
        J => stdctl::LEFT,
        L => stdctl::RIGHT,
        Z => stdctl::A,
        X => stdctl::B,
        Return => stdctl::START,
        S => stdctl::SELECT,
        Up => stdctl::UP,
        Down => stdctl::DOWN,
        Left => stdctl::LEFT,
        Right => stdctl::RIGHT,
        _ => 0,
    }
}

fn joystick_mapping(button: sdl2::controller::Button) -> u8 {
    use sdl2::controller::Button::*;
    match button {
        DPadUp => stdctl::UP,
        DPadDown => stdctl::DOWN,
        DPadLeft => stdctl::LEFT,
        DPadRight => stdctl::RIGHT,
        A => stdctl::A,
        B => stdctl::B,
        X => stdctl::A,
        Y => stdctl::B,
        Start => stdctl::START,
        _ => stdctl::SELECT
    }
}

impl SDLEventPoller {
    fn new(_events: sdl2::EventPump) -> Self {
        SDLEventPoller {
            events: RefCell::new(_events),
            p1_button_state: Cell::new(0),
            exit_flag: Cell::new(false)
        }
    }

    #[inline]
    fn is_exiting(&self) -> bool {
        self.exit_flag.get()
    }
}

impl InputPoller for SDLEventPoller {
    #[inline]
    fn poll(&self) -> u8 {
        use sdl2::keyboard::Keycode::Escape;
        use sdl2::event::Event;
        let mut ns = self.p1_button_state.get();
        for event in self.events.borrow_mut().poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Escape), .. } => {
                    self.exit_flag.set(true)
                },
                Event::KeyDown { keycode: Some(c), .. } =>
                    ns |= keyboard_mapping(c),
                Event::KeyUp { keycode: Some(c), .. } =>
                    ns &= !keyboard_mapping(c),
                Event::ControllerButtonDown { button, .. } =>
                    ns |= joystick_mapping(button),
                Event::ControllerButtonUp { button, .. } =>
                    ns &= !joystick_mapping(button),
                /* TODO: support axis motion
                Event::ControllerAxisMotion { axis: LeftX, value: val, .. } => {
                    let threshold = 10_000;
                        if val > threshold {
                        println!("{}", val);
                            ns |= stdctl::RIGHT;
                            ns &= !stdctl::LEFT;
                        } else if val < -threshold {
                        println!("{}", val);
                            ns |= stdctl::LEFT;
                            ns &= !stdctl::RIGHT;
                        } else {
                            ns &= !(stdctl::RIGHT | stdctl::LEFT);
                        }
                },
                */
                _ => ()
            }
        }
        self.p1_button_state.set(ns);
        ns
    }
}

struct SDLWindow<'a> {
    canvas: sdl2::render::WindowCanvas,
    frame_buffer: [u8; FB_SIZE],
    texture: sdl2::render::Texture,
    copy_area: Option<sdl2::rect::Rect>,
    event: &'a SDLEventPoller
}

impl<'a> SDLWindow<'a> {
    fn new(video_subsystem: &sdl2::VideoSubsystem,
           event: &'a SDLEventPoller,
           pixel_scale: u32,
           full_screen: bool) -> Self {
        let mut actual_height = PIX_HEIGHT * pixel_scale;
        let actual_width = PIX_WIDTH * pixel_scale;
        let mut copy_area = None;
        if !full_screen {
            actual_height -= 16 * pixel_scale;
            copy_area = Some(sdl2::rect::Rect::new(0, 8, PIX_WIDTH, PIX_HEIGHT - 16));
        }
        let window = video_subsystem.window("RuNES", actual_width, actual_height)
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
        SDLWindow {
            canvas,
            frame_buffer: [0; FB_SIZE],
            texture: texture_creator.create_texture_streaming(
                        sdl2::pixels::PixelFormatEnum::RGB24,
                        PIX_WIDTH, PIX_HEIGHT).unwrap(),
            event,
            copy_area
        }
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
        self.canvas.copy(&self.texture, self.copy_area, None).unwrap();
        self.canvas.present();
        self.event.poll();
    }
}

struct CircularBuffer {
    buffer: [i16; (AUDIO_ALL_SAMPLES + 1) as usize],
    head: usize,
    tail: usize
}

impl CircularBuffer {
    fn new() -> Self {
        CircularBuffer {
            buffer: [0; (AUDIO_ALL_SAMPLES + 1) as usize],
            head: 1,
            tail: 0
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
            /*
            let l1 = (b.tail + b.buffer.len() - b.head) % b.buffer.len();
            print!("{} ", l1);
            */
            for x in out.iter_mut() {
                *x = b.deque()
            }
        }
        //println!("{}", m.1);
        if m.1 >= AUDIO_SAMPLES {
            m.1 -= AUDIO_SAMPLES;
            self.0.time_barrier.notify_one();
        } else {
            println!("audio frame skipping {}", m.1);
            m.1 = 0;
        }
    }
}

impl<'a> apu::Speaker for SDLAudio<'a> {
    fn queue(&mut self, sample: i16) {
        let mut m = self.0.buffer.lock().unwrap();
        {
            let b = &mut m.0;
            b.enque(sample);
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
             cpu.get_a(), cpu.get_x(), cpu.get_y(),
             cpu.get_status(), cpu.get_sp());
}

fn main() {
    let matches =
        App::new("RuNES")
            .version("0.1")
            .author("Ted Yin <tederminant@gmail.com>")
            .about("A Rust NES emulator")
            .arg(Arg::with_name("scale")
                 .short("x")
                 .long("scale")
                 .help("Set pixel scaling factor (3 by default)")
                 .required(false)
                 .takes_value(true))
            .arg(Arg::with_name("full")
                 .help("Enable the entire PPU rendering area")
                 .short("f")
                 .long("full")
                 .required(false)
                 .takes_value(false))
            .arg(Arg::with_name("INPUT")
                 .help("iNES ROM file")
                 .required(true)
                 .index(1))
            .arg(Arg::with_name("load")
                 .help("Load from specified machine state file")
                 .short("l")
                 .long("load")
                 .required(false)
                 .takes_value(true))
            .arg(Arg::with_name("save")
                 .help("Save to specified machine state file when exit")
                 .short("s")
                 .long("save")
                 .required(false)
                 .takes_value(true))
            .arg(Arg::with_name("load-sram")
                 .help("Load from specified sram file")
                 .short("L")
                 .long("load-sram")
                 .required(false)
                 .takes_value(true))
            .arg(Arg::with_name("save-sram")
                 .help("Save to specified sram file when exit")
                 .short("S")
                 .long("save-sram")
                 .required(false)
                 .takes_value(true))
            .arg(Arg::with_name("no-state")
                 .help("Power up the emulator with initial state")
                 .short("n")
                 .long("no-state")
                 .required(false)
                 .takes_value(false))
            .get_matches();

    let scale = std::cmp::min(8,
                    std::cmp::max(1,
                        value_t!(matches, "scale", u32).unwrap_or(3)));
    let full = matches.is_present("full");

    let fname = matches.value_of("INPUT").unwrap();
    let load_state_name = matches.value_of("load");
    let save_state_name = matches.value_of("save");
    let save_sram_name = matches.value_of("save-sram");
    let load_sram_name = matches.value_of("load-sram");
    let default_state_name = fname.to_string() + ".runes";
    let default_sram_name = fname.to_string() + ".runes_sram";
    let no_state = matches.is_present("no-state");

    /* load and parse iNES file */
    let mut file = File::open(fname).unwrap();
    let mut rheader = [0; 16];
    file.read(&mut rheader[..]).unwrap();
    let header = unsafe{transmute::<[u8; 16], INesHeader>(rheader)};
    let mirror = match ((header.flags6 >> 2) & 2) | (header.flags6 & 1) {
        0 => MirrorType::Horizontal,
        1 => MirrorType::Vertical,
        2 => MirrorType::Single0,
        3 => MirrorType::Single1,
        _ => MirrorType::Four,
    };
    let mapper_id = (header.flags7 & 0xf0) | (header.flags6 >> 4);
    if std::str::from_utf8(&header.magic).unwrap() != "NES\x1a" {
        println!("not an iNES file");
        exit(1);
    }
    println!("prg size:{}, chr size:{}, mirror type:{}, mapper:{}",
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
    let sram = vec![0; 0x2000];
    println!("read prg {}", file.read(&mut prg_rom[..]).unwrap());
    println!("read chr {}", file.read(&mut chr_rom[..]).unwrap());

    /* setup SDL */
    let sdl_context = sdl2::init().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    /* audio */
    let audio_sync = AudioSync { time_barrier: Condvar::new(),
                                 buffer: Mutex::new((CircularBuffer::new(), AUDIO_ALL_SAMPLES))};
    let mut spkr = SDLAudio(&audio_sync);
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(apu::AUDIO_SAMPLE_FREQ as i32),
        channels: Some(1),
        samples: Some(AUDIO_SAMPLES)
    };
    let device = audio_subsystem.open_playback(None, &desired_spec, |_| {
        SDLAudioPlayback(&audio_sync)
    }).unwrap();

    /* joysticks */
    let njoysticks = match controller_subsystem.num_joysticks() {
        Ok(n)  => n,
        Err(e) => {
            println!("can't enumerate joysticks: {}", e);
            0
        },
    };
    println!("detected {} joysticks", njoysticks);
    let mut _sdl_joystick = None;
    for id in 0..njoysticks {
        if controller_subsystem.is_game_controller(id) {
            match controller_subsystem.open(id) {
                Ok(ctl) => {
                    println!("opened controller {}", ctl.name());
                    println!("controller mapping: {}", ctl.mapping());
                    _sdl_joystick = Some(ctl);
                    break;
                },
                Err(e) => println!("failed to open {}: {}", id, e)
            }
        }
    }

    let event = SDLEventPoller::new(sdl_context.event_pump().unwrap());
    let mut win = SDLWindow::new(&video_subsystem, &event, scale, full);

    /* construct mapper from cartridge data */
    let cart = SimpleCart::new(chr_rom, prg_rom, sram, mirror);
    let mut m: Box<mapper::Mapper> = match mapper_id {
        0 | 2 => Box::new(mapper::Mapper2::new(cart)),
        1 => Box::new(mapper::Mapper1::new(cart)),
        4 => Box::new(mapper::Mapper4::new(cart)),
        _ => panic!("unsupported mapper {}", mapper_id)
    };

    /* controller for player 1 */
    let p1ctl = stdctl::Joystick::new(&event);

    /* setup the emulated machine */
    let mapper = mapper::RefMapper::new(&mut (*m) as &mut mapper::Mapper);
    let mut cpu = CPU::new(CPUMemory::new(&mapper, Some(&p1ctl), None));
    let mut ppu = PPU::new(PPUMemory::new(&mapper), &mut win);
    let mut apu = APU::new(&mut spkr);
    let cpu_ptr = &mut cpu as *mut CPU;
    cpu.mem.bus.attach(cpu_ptr, &mut ppu, &mut apu);

    let load_state = !no_state && match match load_state_name {
        Some(s) => Some(File::open(s).unwrap()),
        None => match File::open(&default_state_name) {
            Ok(file) => Some(file),
            Err(_) => None
        }
    } {
        Some(f) => {
            let mut file = FileIO(f);
            cpu.load(&mut file);
            ppu.load(&mut file);
            apu.load(&mut file);
            mapper.get_mut().load(&mut file);
            true
        },
        None => false
    };

    if !load_state {
        if let Some(f) = match load_sram_name {
            Some(s) => Some(File::open(s).unwrap()),
            None => match File::open(&default_sram_name) {
                Ok(file) => Some(file),
                Err(_) => None
            }
        } {
            let mut file = FileIO(f);
            mapper.get_mut().get_cart_mut().load_sram(&mut file);
        }
        cpu.powerup()
    }

    device.resume();
    loop {
        /* consume the leftover cycles from the last instruction */
        while cpu.cycle > 0 {
            cpu.mem.bus.tick()
        }

        if event.is_exiting() {
            {
                let mut file = FileIO(File::create(match save_state_name {
                    Some(s) => s.to_string(),
                    None => default_state_name
                }).unwrap());
                cpu.save(&mut file);
                ppu.save(&mut file);
                apu.save(&mut file);
                mapper.save(&mut file);
            }
            {
                let mut file = FileIO(File::create(match save_sram_name {
                    Some(s) => s.to_string(),
                    None => default_sram_name
                }).unwrap());
                mapper.get_cart().save_sram(&mut file);
            }
            exit(0);
        }
        //print_cpu_trace(&cpu);
        cpu.step();
    }
}
