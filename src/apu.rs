#![allow(dead_code)]
use mos6502;

pub trait Speaker {
    fn queue(&mut self, sample: u16);
    fn push(&mut self);
}

const CPU_SAMPLE_FREQ: u32 = 240;
pub const AUDIO_SAMPLE_FREQ: u32 = 44100;

const LEN_TABLE: [u8; 32] = [
    10, 254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
    12,  16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

const DUTY_TABLE: [u8; 4] = [
    0b00000010,
    0b00000110,
    0b00011110,
    0b11111001,
];

const PULSE_TABLE: [u16; 31] = [
    0x0000, 0x02f8, 0x05df, 0x08b4, 0x0b78, 0x0e2b,
    0x10cf, 0x1363, 0x15e9, 0x1860, 0x1ac9, 0x1d25,
    0x1f75, 0x21b7, 0x23ee, 0x2618, 0x2837, 0x2a4c,
    0x2c55, 0x2e54, 0x3049, 0x3234, 0x3416, 0x35ee,
    0x37be, 0x3985, 0x3b43, 0x3cf9, 0x3ea7, 0x404d,
    0x41ec
];

const TND_TABLE: [u16; 203] = [
    0x0000, 0x01b7, 0x036a, 0x051a, 0x06c6, 0x086f,
    0x0a15, 0x0bb7, 0x0d56, 0x0ef2, 0x108a, 0x121f,
    0x13b1, 0x1540, 0x16cc, 0x1855, 0x19da, 0x1b5d,
    0x1cdd, 0x1e59, 0x1fd3, 0x214a, 0x22be, 0x2430,
    0x259e, 0x270a, 0x2874, 0x29da, 0x2b3e, 0x2c9f,
    0x2dfe, 0x2f5a, 0x30b4, 0x320b, 0x335f, 0x34b2,
    0x3601, 0x374f, 0x389a, 0x39e2, 0x3b29, 0x3c6d,
    0x3dae, 0x3eee, 0x402b, 0x4166, 0x429f, 0x43d6,
    0x450a, 0x463d, 0x476d, 0x489c, 0x49c8, 0x4af2,
    0x4c1b, 0x4d41, 0x4e65, 0x4f87, 0x50a8, 0x51c6,
    0x52e3, 0x53fe, 0x5517, 0x562e, 0x5743, 0x5856,
    0x5968, 0x5a78, 0x5b86, 0x5c93, 0x5d9d, 0x5ea6,
    0x5fae, 0x60b3, 0x61b7, 0x62ba, 0x63bb, 0x64ba,
    0x65b7, 0x66b3, 0x67ae, 0x68a7, 0x699e, 0x6a94,
    0x6b88, 0x6c7b, 0x6d6d, 0x6e5d, 0x6f4b, 0x7038,
    0x7124, 0x720e, 0x72f7, 0x73de, 0x74c4, 0x75a9,
    0x768c, 0x776e, 0x784f, 0x792e, 0x7a0d, 0x7ae9,
    0x7bc5, 0x7c9f, 0x7d78, 0x7e50, 0x7f26, 0x7ffc,
    0x80d0, 0x81a3, 0x8274, 0x8345, 0x8414, 0x84e2,
    0x85af, 0x867b, 0x8746, 0x880f, 0x88d8, 0x899f,
    0x8a65, 0x8b2b, 0x8bef, 0x8cb2, 0x8d74, 0x8e35,
    0x8ef4, 0x8fb3, 0x9071, 0x912e, 0x91ea, 0x92a4,
    0x935e, 0x9417, 0x94cf, 0x9586, 0x963c, 0x96f0,
    0x97a4, 0x9857, 0x990a, 0x99bb, 0x9a6b, 0x9b1a,
    0x9bc9, 0x9c76, 0x9d23, 0x9dcf, 0x9e7a, 0x9f24,
    0x9fcd, 0xa075, 0xa11c, 0xa1c3, 0xa269, 0xa30e,
    0xa3b2, 0xa455, 0xa4f7, 0xa599, 0xa63a, 0xa6da,
    0xa779, 0xa818, 0xa8b5, 0xa952, 0xa9ef, 0xaa8a,
    0xab25, 0xabbe, 0xac58, 0xacf0, 0xad88, 0xae1f,
    0xaeb5, 0xaf4a, 0xafdf, 0xb073, 0xb107, 0xb199,
    0xb22b, 0xb2bd, 0xb34d, 0xb3dd, 0xb46c, 0xb4fb,
    0xb589, 0xb616, 0xb6a3, 0xb72f, 0xb7ba, 0xb845,
    0xb8cf, 0xb958, 0xb9e1, 0xba69, 0xbaf1, 0xbb78,
    0xbbfe, 0xbc84, 0xbd09, 0xbd8d, 0xbe11
];

struct Sampler {
    ticks_remain: u32,
    ticks_now: u32,
    ticks_unit: u32,
    ticks_all: u32,
    ticks_extra: u32,
    ticks_extra_all: u32,
}

impl Sampler {
    fn new(freq1: u32, freq2: u32) -> Self {
        let unit = freq1 / freq2;
        let extra = freq1 - unit * freq2;
        Sampler {
            ticks_remain: freq1 - extra,
            ticks_now: 0,
            ticks_unit: unit,
            ticks_all: freq1 - extra,
            ticks_extra: extra,
            ticks_extra_all: extra
        }
    }

    fn tick(&mut self) -> (bool, bool) {
        let unit = self.ticks_unit;
        if self.ticks_now == 0 {
            self.ticks_now = unit;
            self.ticks_remain -= unit;
            if self.ticks_remain == 0 {
                /* compensate to last exactly 1 sec */
                self.ticks_now += self.ticks_remain;
                /* reload for the next second */
                self.ticks_remain = self.ticks_all;
                self.ticks_extra = self.ticks_extra_all;
            }
            if self.ticks_extra > 0 {
                self.ticks_extra -= 1;
                self.ticks_now += 1;
            }
        }
        self.ticks_now -= 1;
        (self.ticks_now == 0, self.ticks_remain == self.ticks_all)
    }
}

pub struct Pulse {
    /* envelope */
    env_period: u8,
    env_lvl: u8,
    decay_lvl: u8,
    env_start: bool,
    env_loop: bool,
    env_const: bool,
    env_vol: u8,
    /* sweep */
    swp_count: u8,
    swp_period: u8,
    swp_lvl: u8,
    swp_en: bool,
    swp_neg: bool,
    swp_rld: bool,
    muted: bool,
    /* length counter */
    len_lvl: u8,
    /* timer */
    timer_period: u16,
    timer_lvl: u16,
    /* sequencer */
    seq_wave: u8,
    seq_cnt: u8,
    /* channel */
    enabled: bool,
    comple: bool,
}

impl Pulse {
    fn tick_env(&mut self) {
        /* should be clocked by frame counter */
        if !self.env_start {
            if self.env_lvl == 0 {
                self.env_lvl = self.env_period;
                if self.decay_lvl == 0 {
                    if self.env_loop {
                        self.decay_lvl = 15;
                    }
                } else {
                    self.decay_lvl -= 1;
                }
            } else {
                self.env_lvl -= 1;
            }
        } else {
            self.decay_lvl = 15;
            self.env_start = false;
            self.env_lvl = self.env_period;
        }
    }

    fn tick_sweep(&mut self) {
        let mut reload = self.swp_rld;
        if self.swp_lvl == 0 {
            reload = true;
            let p = self.timer_period;
            let mut delta = p >> self.swp_count;
            if self.swp_neg {
                delta = !delta;
                if self.comple { delta += 1; } /* two's complement */
            }
            p.wrapping_add(delta);
            self.muted = self.timer_period < 8 || (self.timer_period >> 11 != 0);
            if !self.muted && self.swp_en && self.swp_count != 0 {
                self.timer_period = p;
            }
        } else {
            self.swp_lvl -= 1;
        }
        if reload {
            self.swp_lvl = self.swp_period;
            self.swp_rld = false;
        }
    }

    fn disable(&mut self) {
        self.len_lvl = 0;
        self.enabled = false;
    }

    fn enable(&mut self) { self.enabled = true }

    fn tick_length(&mut self) {
        if self.len_lvl > 0 && !self.env_loop {
            self.len_lvl -= 1
        }
    }

    fn tick_timer(&mut self) {
        if self.timer_lvl == 0 {
            self.timer_lvl = self.timer_period;
            if self.seq_cnt == 7 {
                self.seq_cnt = 0
            } else {
                self.seq_cnt += 1
            }
        } else {
            self.timer_lvl -= 1
        }
    }

    fn get_len(&self) -> u8 { self.len_lvl }
    fn set_duty(&mut self, b: u8) { self.seq_wave = DUTY_TABLE[b as usize] }
    fn set_loop(&mut self, b: bool) { self.env_loop = b }
    fn set_const(&mut self, b: bool) { self.env_const = b }
    fn set_env_period(&mut self, p: u8) { self.env_period = p } /* 4 bits */
    fn set_env_vol(&mut self, p: u8) { self.env_vol = p }
    fn set_sweep(&mut self, d: u8) {
        self.swp_en = (d >> 7) == 1;
        self.swp_period = (d >> 4) & 7;
        self.swp_neg = d & 0x8 == 0x8;
        self.swp_count = d & 7;
        self.swp_rld = true;
    }

    fn set_timer_period(&mut self, p: u16) {
        self.muted = p < 8;
        self.timer_period = p;
    }

    fn set_len(&mut self, d: u8) {
        if self.enabled {
            self.len_lvl = LEN_TABLE[d as usize]
        }
    }

    pub fn write_reg1(&mut self, data: u8) {
        self.set_duty(data >> 6);
        self.set_loop(data & 0x20 == 0x20);
        self.set_const(data & 0x10 == 0x10);
        self.set_env_period(data & 0xf);
        self.set_env_vol(data & 0xf);
        self.env_start = true;
    }

    pub fn write_reg2(&mut self, data: u8) { self.set_sweep(data) }

    pub fn write_reg3(&mut self, data: u8) {
        self.timer_period = (self.timer_period & 0xff00) | data as u16
    }

    pub fn write_reg4(&mut self, data: u8) {
        self.set_len(data >> 3);
        self.timer_period = (self.timer_period & 0x00ff) | ((data as u16 & 7) << 8);
        self.seq_cnt = 0;
        self.env_start = true;
    }

    pub fn output(&self) -> u8 {
        let env = if self.env_const { self.env_vol } else { self.decay_lvl };
        let swp = !self.muted;
        let seq = (self.seq_wave >> self.seq_cnt) & 1 == 1;
        let len = self.len_lvl > 0;
        if swp && seq && len { env } else { 0 }
    }
    
    pub fn new(comple: bool) -> Self {
        Pulse {env_period: 0, env_lvl: 0, decay_lvl: 0,
               env_start: false, env_loop: false, env_const: false, env_vol: 0,
               swp_count: 0, swp_period: 0, swp_lvl: 0,
               swp_en: false, swp_neg: false, swp_rld: false, muted: true,
               len_lvl: 0, timer_period: 0, timer_lvl: 0,
               seq_wave: 0, seq_cnt: 0, enabled: false, comple}
    }
}

pub struct APU<'a> {
    pub pulse1: Pulse,
    pub pulse2: Pulse,
    frame_lvl: u8,
    frame_mode: bool, /* true for 5-step mode */
    frame_inh: bool,
    frame_int: bool,
    cpu_sampler: Sampler,
    audio_sampler: Sampler,
    cycle_even: bool,
    spkr: &'a mut Speaker
}

impl<'a> APU<'a> {
    fn tick_env(&mut self) {
        self.pulse1.tick_env();
        self.pulse2.tick_env();
    }

    fn tick_len_swp(&mut self) {
        self.pulse1.tick_length();
        self.pulse1.tick_sweep();
        self.pulse2.tick_length();
        self.pulse2.tick_sweep();
    }
    
    pub fn new(spkr: &'a mut Speaker) -> Self {
        APU {
            pulse1: Pulse::new(false), pulse2: Pulse::new(true),
            frame_lvl: 0, frame_mode: false, frame_int: false, frame_inh: false,
            cpu_sampler: Sampler::new(mos6502::CPU_FREQ, CPU_SAMPLE_FREQ),
            audio_sampler: Sampler::new(mos6502::CPU_FREQ, AUDIO_SAMPLE_FREQ),
            cycle_even: false,
            spkr
        }
    }

    pub fn output(&self) -> u16 {
        let pulse_out = PULSE_TABLE[(self.pulse1.output() +
                                    self.pulse2.output()) as usize];
        let tnd_out = TND_TABLE[0];
        pulse_out + tnd_out
    }

    pub fn read_status(&mut self) -> u8 {
        let res = if self.pulse1.get_len() > 0 { 1 } else { 0 } |
                  (if self.pulse1.get_len() > 0 { 1 } else { 0 }) << 1 |
                  (if self.frame_int { 1 } else { 0 }) << 6;
        if self.frame_lvl != 3 {
            self.frame_int = false; /* clear interrupt flag */
        }
        res
    }

    pub fn write_status(&mut self, data: u8) {
        match data & 0x1 {
            0 => self.pulse1.disable(),
            _ => self.pulse1.enable()
        }
        match data & 0x2 {
            0 => self.pulse2.disable(),
            _ => self.pulse2.enable()
        }
    }

    pub fn write_frame_counter(&mut self, data: u8) {
        self.frame_inh = data & 0x40 == 1;
        self.frame_mode = data >> 7 == 1;
    }

    pub fn tick_timer(&mut self) {
        if self.cycle_even {
            self.pulse1.tick_timer();
            self.pulse2.tick_timer();
        }
    }

    fn tick_frame_counter(&mut self) -> bool {
        let f = self.frame_lvl;
        match self.frame_mode {
            false => {
                match f {
                    0 | 2 => self.tick_env(),
                    1 => {
                        self.tick_env();
                        self.tick_len_swp();
                    },
                    _ => {
                        self.tick_env();
                        self.tick_len_swp();
                        if !self.frame_inh {
                            self.frame_int = true;
                        }
                    },
                };
                self.frame_lvl = if f == 3 { 0 } else { f + 1 }
            },
            true => {
                match f {
                    0 | 2 => self.tick_env(),
                    1 | 4 => {
                        self.tick_env();
                        self.tick_len_swp();
                    },
                    _ => ()
                }
                self.frame_lvl = if f == 4 { 0 } else { f + 1 }
            }
        }
        self.frame_int
    }

    pub fn tick(&mut self) -> bool {
        let mut irq = false;
        if let (true, _) = self.cpu_sampler.tick() {
            irq = self.tick_frame_counter();
            //print!("+");
        }
        if let (true, sec) = self.audio_sampler.tick() {
            let sample = self.output();
            self.spkr.queue(sample);
            //print!(".");
            if sec {
                self.spkr.push();
                //println!("ok");
            }
        }
        self.tick_timer();
        self.cycle_even = !self.cycle_even;
        irq
    }
}
