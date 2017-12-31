#![allow(dead_code)]
use mos6502;

pub trait Speaker {
    fn queue(&mut self, sample: u16);
}

const CPU_SAMPLE_FREQ: u32 = 240;
pub const AUDIO_SAMPLE_FREQ: u32 = 44100;

const TRI_SEQ_TABLE: [u8; 32] = [
    15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15
];

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

const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068
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

pub struct Sampler {
    freq2: u32,
    q0: u32,
    r0: u32,
    ddl: (u32, u32),
    cnt: u32,
    sec_cnt: u32
}

impl Sampler {
    pub fn new(freq1: u32, freq2: u32) -> Self {
        let q0 = freq1 / freq2;
        let r0 = freq1 - q0 * freq2;
        Sampler {
            freq2,
            q0,
            r0,
            ddl: (q0, r0),
            cnt: 0,
            sec_cnt: 0
        }
    }

    pub fn tick(&mut self) -> (bool, bool) {
        let (q, r) = self.ddl;
        if self.cnt == q {
            let nr = r + self.r0;
            self.ddl = if nr > self.freq2 {
                (self.q0, nr - self.freq2)
            } else {
                (self.q0 - 1, nr)
            };
            self.cnt = 0;
            self.sec_cnt += 1;
            let sec = self.sec_cnt == self.freq2;
            if sec {
                self.sec_cnt = 0
            }
            (true, sec)
        } else {
            self.cnt += 1;
            (false, false)
        }
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
    pub fn new(comple: bool) -> Self {
        Pulse {env_period: 0, env_lvl: 0, decay_lvl: 0,
               env_start: false, env_loop: false, env_const: false, env_vol: 0,
               swp_count: 0, swp_period: 0, swp_lvl: 0,
               swp_en: false, swp_neg: false, swp_rld: false, muted: false,
               len_lvl: 0, timer_period: 0, timer_lvl: 0,
               seq_wave: 0, seq_cnt: 0, enabled: false, comple}
    }

    pub fn write_reg1(&mut self, data: u8) {
        self.seq_wave = DUTY_TABLE[(data >> 6) as usize];
        self.env_loop = data & 0x20 == 0x20;
        self.env_const = data & 0x10 == 0x10;
        self.env_period = data & 0xf;
        self.env_vol = data & 0xf;
    }

    pub fn write_reg2(&mut self, data: u8) {
        self.swp_en = (data >> 7) == 1;
        self.swp_period = (data >> 4) & 7;
        self.swp_neg = data & 0x8 == 0x8;
        self.swp_count = data & 7;
        self.swp_rld = true;
    }

    pub fn write_reg3(&mut self, data: u8) {
        let p = (self.timer_period & 0xff00) | data as u16;
        self.set_timer_period(p);
    }

    pub fn write_reg4(&mut self, data: u8) {
        self.set_len(data >> 3);
        let p = (self.timer_period & 0x00ff) | ((data as u16 & 7) << 8);
        self.set_timer_period(p);
        self.seq_cnt = 0;
        self.env_start = true;
    }

    pub fn output(&self) -> u8 {
        let env = if self.env_const { self.env_vol } else { self.decay_lvl };
        let swp = !self.muted;
        let seq = (self.seq_wave >> self.seq_cnt) & 1 == 1;
        let len = self.len_lvl > 0;
        if self.enabled && swp && seq && len { env } else { 0 }
    }

    fn tick_env(&mut self) {
        if !self.env_start {
            if self.env_lvl == 0 {
                self.env_lvl = self.env_period;
                if self.decay_lvl == 0 {
                    if self.env_loop {
                        self.decay_lvl = 0xf;
                    }
                } else {
                    self.decay_lvl -= 1;
                }
            } else {
                self.env_lvl -= 1;
            }
        } else {
            self.decay_lvl = 0xf;
            self.env_start = false;
            self.env_lvl = self.env_period;
        }
    }

    fn tick_sweep(&mut self) {
        let mut reload = self.swp_rld;
        if self.swp_lvl == 0 {
            reload = true;
            if self.swp_en {
                let mut p: u16 = self.timer_period;
                let mut delta = p >> self.swp_count;
                if self.swp_neg {
                    delta = !delta;
                    if self.comple { delta += 1; } /* two's complement */
                }
                p = p.wrapping_add(delta);
                self.muted = p < 8 || (p >> 11 != 0);
                if !self.muted && self.swp_count != 0 {
                    self.timer_period = p;
                }
            }
        } else {
            self.swp_lvl -= 1;
        }
        if reload {
            self.swp_lvl = self.swp_period;
            self.swp_rld = false;
        }
    }

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

    #[inline(always)]
    fn disable(&mut self) {
        self.len_lvl = 0;
        self.enabled = false;
    }

    #[inline(always)]
    fn enable(&mut self) { self.enabled = true }

    #[inline(always)] fn get_len(&self) -> u8 { self.len_lvl }

    #[inline(always)]
    fn set_len(&mut self, d: u8) {
        if self.enabled {
            self.len_lvl = LEN_TABLE[d as usize]
        }
    }

    #[inline(always)]
    fn set_timer_period(&mut self, p: u16) {
        self.muted = p < 8;
        self.timer_period = p;
    }
}

pub struct Triangle {
    /* linear counter */
    cnt_rld: bool,
    cnt_lvl: u8,
    cnt_rld_val: u8,
    /* length counter */
    len_lvl: u8,
    /* timer */
    timer_period: u16,
    timer_lvl: u16,
    /* sequencer */
    seq_cnt: u8,
    enabled: bool,
    /* misc */
    ctrl: bool
}

impl Triangle {
    fn new() -> Self {
        Triangle {
            cnt_rld: false, cnt_lvl: 0, cnt_rld_val: 0,
            len_lvl: 0, timer_period: 0, timer_lvl: 0,
            seq_cnt: 0, enabled: false, ctrl: false
        }
    }

    pub fn write_reg1(&mut self, data: u8) {
        self.cnt_rld_val = data & 0x7f;
        self.ctrl = data >> 7 == 1;
    }

    pub fn write_reg3(&mut self, data: u8) {
        self.timer_period = (self.timer_period & 0xff00) | data as u16;
    }

    pub fn write_reg4(&mut self, data: u8) {
        self.set_len(data >> 3);
        self.timer_period = (self.timer_period & 0x00ff) | ((data as u16 & 7) << 8);
        self.timer_lvl = self.timer_period;
        self.cnt_rld = true;
    }

    pub fn output(&self) -> u8 {
        if self.enabled { TRI_SEQ_TABLE[self.seq_cnt as usize] } else { 0 }
    }

    fn tick_counter(&mut self) {
        if self.cnt_rld {
            self.cnt_lvl = self.cnt_rld_val
        } else if self.cnt_lvl > 0 {
            self.cnt_lvl -= 1
        }
        if !self.ctrl {
            self.cnt_rld = false
        }
    }

    fn tick_length(&mut self) {
        if self.len_lvl > 0 && !self.ctrl {
            self.len_lvl -= 1
        }
    }


    fn tick_timer(&mut self) {
        if self.len_lvl > 0 && self.cnt_lvl > 0 {
            if self.timer_lvl == 0 {
                self.timer_lvl = self.timer_period;
                if self.seq_cnt == 31 {
                    self.seq_cnt = 0
                } else {
                    self.seq_cnt += 1
                }
            } else {
                self.timer_lvl -= 1
            }
        }
    }

    #[inline(always)]
    fn disable(&mut self) {
        self.len_lvl = 0;
        self.enabled = false;
    }

    #[inline(always)]
    fn enable(&mut self) { self.enabled = true }

    #[inline(always)] fn get_len(&self) -> u8 { self.len_lvl }
    
    #[inline(always)]
    fn set_len(&mut self, d: u8) {
        if self.enabled {
            self.len_lvl = LEN_TABLE[d as usize]
        }
    }
}


pub struct Noise {
    /* envelope */
    env_period: u8,
    env_lvl: u8,
    decay_lvl: u8,
    env_start: bool,
    env_loop: bool,
    env_const: bool,
    env_vol: u8,
    /* length counter */
    len_lvl: u8,
    /* timer */
    timer_period: u16,
    timer_lvl: u16,
    /* rng */
    shift_reg: u16,
    loop_noise: bool,
    /* channel */
    enabled: bool,
}

impl Noise {
    pub fn new() -> Self {
        Noise {env_period: 0, env_lvl: 0, decay_lvl: 0,
               env_start: false, env_loop: false, env_const: false, env_vol: 0,
               len_lvl: 0, timer_period: 0, timer_lvl: 0,
               shift_reg: 1, loop_noise: false,
               enabled: false}
    }

    pub fn write_reg1(&mut self, data: u8) {
        self.env_loop = data & 0x20 == 0x20;
        self.env_const = data & 0x10 == 0x10;
        self.env_period = data & 0xf;
        self.env_vol = data & 0xf;
    }

    pub fn write_reg3(&mut self, data: u8) {
        self.loop_noise = (data >> 7) == 1;
        self.timer_period = NOISE_PERIOD_TABLE[data as usize & 0xf];
    }

    pub fn write_reg4(&mut self, data: u8) {
        self.set_len(data >> 3);
        self.env_start = true;
    }

    pub fn output(&self) -> u8 {
        let env = if self.env_const { self.env_vol } else { self.decay_lvl };
        let len = self.len_lvl > 0;
        let shift = self.shift_reg & 1 == 0;
        if self.enabled && shift && len { env } else { 0 }
    }

    fn tick_env(&mut self) {
        if !self.env_start {
            if self.env_lvl == 0 {
                self.env_lvl = self.env_period;
                if self.decay_lvl == 0 {
                    if self.env_loop {
                        self.decay_lvl = 0xf;
                    }
                } else {
                    self.decay_lvl -= 1;
                }
            } else {
                self.env_lvl -= 1;
            }
        } else {
            self.decay_lvl = 0xf;
            self.env_start = false;
            self.env_lvl = self.env_period;
        }
    }

    fn tick_length(&mut self) {
        if self.len_lvl > 0 && !self.env_loop {
            self.len_lvl -= 1
        }
    }

    fn tick_timer(&mut self) {
        if self.timer_lvl == 0 {
            self.timer_lvl = self.timer_period;
            /* shift register is clocked */
            let bit = if self.loop_noise {6} else {1};
            let feedback = (self.shift_reg & 1) ^ ((self.shift_reg >> bit) & 1);
            self.shift_reg = (self.shift_reg >> 1) | (feedback << 14);
        } else {
            self.timer_lvl -= 1
        }
    }

    #[inline(always)]
    fn disable(&mut self) {
        self.len_lvl = 0;
        self.enabled = false;
    }

    #[inline(always)]
    fn enable(&mut self) { self.enabled = true }

    #[inline(always)] fn get_len(&self) -> u8 { self.len_lvl }

    #[inline(always)]
    fn set_len(&mut self, d: u8) {
        if self.enabled {
            self.len_lvl = LEN_TABLE[d as usize]
        }
    }
}


pub struct APU<'a> {
    pub pulse1: Pulse,
    pub pulse2: Pulse,
    pub triangle: Triangle,
    pub noise: Noise,
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
    pub fn new(spkr: &'a mut Speaker) -> Self {
        APU {
            pulse1: Pulse::new(false), pulse2: Pulse::new(true),
            triangle: Triangle::new(),
            noise: Noise::new(),
            frame_lvl: 0, frame_mode: false, frame_int: false, frame_inh: true,
            cpu_sampler: Sampler::new(mos6502::CPU_FREQ, CPU_SAMPLE_FREQ),
            audio_sampler: Sampler::new(mos6502::CPU_FREQ, AUDIO_SAMPLE_FREQ),
            cycle_even: false,
            spkr
        }
    }

    pub fn tick(&mut self) -> bool {
        let mut irq = false;
        if let (true, _) = self.cpu_sampler.tick() {
            irq = self.tick_frame_counter();
        }
        if let (true, _) = self.audio_sampler.tick() {
            let sample = self.output();
            self.spkr.queue(sample);
        }
        self.tick_timer();
        self.cycle_even = !self.cycle_even;
        irq
    }

    pub fn output(&self) -> u16 {
        let pulse_out = PULSE_TABLE[(self.pulse1.output() +
                                    self.pulse2.output()) as usize];
        let tnd_out = TND_TABLE[(self.triangle.output() * 3 +
                                self.noise.output() * 2) as usize];
        pulse_out + tnd_out
    }

    pub fn read_status(&mut self) -> u8 {
        let res = if self.pulse1.get_len() > 0 { 1 } else { 0 } |
                  (if self.pulse2.get_len() > 0 { 1 } else { 0 }) << 1 |
                  (if self.triangle.get_len() > 0 { 1 } else { 0 }) << 2 |
                  (if self.noise.get_len() > 0 { 1 } else { 0 }) << 3 |
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
        match data & 0x4 {
            0 => self.triangle.disable(),
            _ => self.triangle.enable()
        }
        match data & 0x8 {
            0 => self.noise.disable(),
            _ => self.noise.enable()
        }
    }

    pub fn write_frame_counter(&mut self, data: u8) {
        self.frame_inh = data & 0x40 == 0x40;
        self.frame_mode = data >> 7 == 1;
        if self.frame_mode {
            self.tick_len_swp()
        }
    }

    fn tick_timer(&mut self) {
        if self.cycle_even {
            self.pulse1.tick_timer();
            self.pulse2.tick_timer();
            self.noise.tick_timer();
        }
        self.triangle.tick_timer();
    }

    fn tick_env_cnt(&mut self) {
        self.pulse1.tick_env();
        self.pulse2.tick_env();
        self.triangle.tick_counter();
        self.noise.tick_env();
    }

    fn tick_len_swp(&mut self) {
        self.pulse1.tick_length();
        self.pulse1.tick_sweep();
        self.pulse2.tick_length();
        self.pulse2.tick_sweep();
        self.triangle.tick_length();
        self.noise.tick_length();
    }

    fn tick_frame_counter(&mut self) -> bool {
        let f = self.frame_lvl;
        match self.frame_mode {
            false => {
                self.frame_lvl = if f == 3 { 0 } else { f + 1 };
                match f {
                    1 | 3 => self.tick_env_cnt(),
                    2 => {
                        self.tick_env_cnt();
                        self.tick_len_swp();
                    },
                    _ => {
                        self.tick_env_cnt();
                        self.tick_len_swp();
                        if !self.frame_inh {
                            self.frame_int = true;
                        }
                    },
                };
            },
            true => {
                self.frame_lvl = if f == 4 { 0 } else { f + 1 };
                match f {
                    1 | 3 => self.tick_env_cnt(),
                    0 | 2 => {
                        self.tick_env_cnt();
                        self.tick_len_swp();
                    },
                    _ => self.tick_env_cnt()
                }
            }
        }
        self.frame_int
    }
}
