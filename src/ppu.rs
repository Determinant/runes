#![allow(dead_code)]
use memory::VMem;
use mos6502::CPU;
use core::intrinsics::transmute;

pub trait Screen {
    fn put(&self, x: u8, y: u8, color: u8);
    fn render(&self);
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct Sprite {
    y: u8,      /* is the (actualy y) - 1 */
    tile: u8,
    attr: u8,
    x: u8
}

pub struct PPU<'a> {
    scanline: u16,
    /* registers */
    ppuctl: u8,
    ppumask: u8,
    ppustatus: u8,
    oamaddr: u8,

    reg: u8,
    
    x: u8, /* fine x scroll */
    v: u16, /* current vram addr */
    t: u16, /* temporary vram addr */
    w: bool, /* first/second write toggle */
    cycle: u16, /* cycle in the current scanline */
    /* rendering regs & latches */
        /* background registers */
    bg_bitmap: [u16; 2],
    bg_palette: [u16; 2],
        /* background latches */
    bg_nt: u8,
    bg_attr: u8,
    bg_bit_low: u8,
    bg_bit_high: u8,
    /* sprites */
    oam: [Sprite; 64],
    oam2: [usize; 8],
    sp_bitmap: [[u8; 2]; 8],
    sp_idx: [usize; 8],
    sp_cnt: [u8; 8],
    sp_zero_insight: bool,
    rendering: bool,
    buffered_read: u8,
    /* IO */
    mem: &'a VMem,
    scr: &'a Screen,
}

impl<'a> PPU<'a> {
    pub fn write_ctl(&mut self, data: u8) {
        self.ppuctl = data;
        self.t = (self.t & 0x73ff) | ((data as u16 & 3) << 10);
    }

    pub fn write_mask(&mut self, data: u8) {
        self.ppumask = data;
    }

    pub fn read_status(&mut self) -> u8 {
        let res = (self.ppustatus & !0x1fu8) | (self.reg & 0x1f);
        self.ppustatus &= !PPU::FLAG_VBLANK;
        self.w = false;
        res
    }

    pub fn write_oamaddr(&mut self, data: u8) {
        self.oamaddr = data;
    }

    pub fn write_oamdata(&mut self, data: u8) {
        if self.rendering { return }
        unsafe {
            let oam_raw = transmute::<&mut[Sprite; 64], &mut[u8; 256]>(&mut self.oam);
            oam_raw[self.oamaddr as usize] = data;
            self.oamaddr = self.oamaddr.wrapping_add(1);
        }
    }

    pub fn read_oamdata(&self) -> u8 {
        unsafe {
            let oam_raw = transmute::<&[Sprite; 64], &[u8; 256]>(&self.oam);
            oam_raw[self.oamaddr as usize]
        }
    }

    pub fn write_scroll(&mut self, data: u8) {
        let data = data as u16;
        match self.w {
            false => {
                self.t = (self.t & 0x7fe0) | (data >> 3);
                self.x = (data & 0x07) as u8;
                assert!(self.x == 0);
                self.w = true;
            },
            true => {
                self.t = (self.t & 0x0c1f) | ((data & 0xf8) << 2) | ((data & 0x07) << 12);
                self.w = false;
            }
        }
    }

    pub fn write_addr(&mut self, data: u8) {
        let data = data as u16;
        match self.w {
            false => {
                self.t = (self.t & 0x00ff) | ((data & 0x3f) << 8);
                self.w = true;
            },
            true => {
                self.t = (self.t & 0xff00) | data;
                self.v = self.t;
                self.w = false;
            }
        }
    }

    pub fn read_data(&mut self) -> u8 {
        let data = self.mem.read(self.v);
        let res = if self.v & 0x3fff < 0x3f00 {
            let prev = self.buffered_read;
            self.buffered_read = data;
            prev
        } else {
            self.buffered_read = self.mem.read(self.v - 0x1000);
            data
        };
        self.v = self.v.wrapping_add(match self.get_vram_inc() {
            0 => 1,
            _ => 32
        });
        res
    }

    pub fn write_data(&mut self, data: u8) {
        self.mem.write(self.v, data);
        self.v = self.v.wrapping_add(match self.get_vram_inc() {
            0 => 1,
            _ => 32
        });
    }

    pub fn write_oamdma(&mut self, data: u8, cpu: &mut CPU) {
        let mut addr = (data as u16) << 8;
        unsafe {
            let oam_raw = transmute::<&mut[Sprite; 64], &mut[u8; 256]>(&mut self.oam);
            for _ in 0..0x100 {
                oam_raw[self.oamaddr as usize] = cpu.mem.read(addr);
                addr = addr.wrapping_add(1);
                self.oamaddr = self.oamaddr.wrapping_add(1);
            }
        }
        cpu.cycle += 1;
        cpu.cycle += cpu.cycle & 1;
        cpu.cycle += 512;
    }

    #[inline(always)] fn get_spritesize(&self) -> u8 {(self.ppuctl >> 5) & 1}
    #[inline(always)] fn get_flag_nmi(&self) -> bool { (self.ppuctl >> 7) == 1 }
    #[inline(always)] fn get_vram_inc(&self) -> u8 { (self.ppuctl >> 2) & 1}
    #[inline(always)] fn get_show_leftmost_bg(&self) -> bool { (self.ppumask >> 1) & 1 == 1}
    #[inline(always)] fn get_show_leftmost_sp(&self) -> bool { (self.ppumask >> 2) & 1 == 1}
    #[inline(always)] fn get_show_bg(&self) -> bool { (self.ppumask >> 3) & 1 == 1}
    #[inline(always)] fn get_show_sp(&self) -> bool { (self.ppumask >> 4) & 1 == 1}
    const FLAG_OVERFLOW: u8 = 1 << 5;
    const FLAG_SPRITE_ZERO: u8 = 1 << 6;
    const FLAG_VBLANK: u8 = 1 << 7;

    #[inline(always)]
    fn fetch_nametable_byte(&mut self) {
        self.bg_nt = self.mem.read(0x2000 | (self.v & 0x0fff));
    }

    #[inline(always)]
    fn fetch_attrtable_byte(&mut self) {
        let v = self.v;
        /* the byte representing 4x4 tiles */
        let b = self.mem.read(0x23c0 | (v & 0x0c00) |
                            ((v >> 4) & 0x38) | ((v >> 2) & 0x07));
        self.bg_attr = (b >> ((v & 2) | ((v & 0x40) >> 4))) & 3;
    }

    #[inline(always)]
    fn fetch_low_bgtile_byte(&mut self) {
                                        /* 0x?000 */
        self.bg_bit_low = self.mem.read(((self.ppuctl as u16 & 0x10) << 8) |
                                        /* 0x-??0 */
                                        ((self.bg_nt as u16) << 4) |
                                        /* 0x---? (0 - 7) */
                                        ((self.v >> 12) & 7) | 0x0);
    }

    #[inline(always)]
    fn fetch_high_bgtile_byte(&mut self) {
                                        /* 0x?000 */
        self.bg_bit_high = self.mem.read(((self.ppuctl as u16 & 0x10) << 8) |
                                        /* 0x-??0 */
                                        ((self.bg_nt as u16) << 4) |
                                        /* 0x---? (8 - f) */
                                        ((self.v >> 12) & 7) | 0x8);
    }

    #[inline(always)]
    fn load_bgtile(&mut self) {
        /* load the tile bitmap to high 8 bits of bitmap,
         * assume the high 8 bits are zeros */
        //if (!(self.bg_bitmap[0] >> 8 == 0 &&
        //        self.bg_bitmap[1] >> 8 == 0 &&
        //        self.bg_palette[0] >> 8 == 0 &&
        //        self.bg_palette[1] >> 8 == 0)) {
        //    println!("at cycle {} {}", self.scanline, self.cycle) }
        //assert!(self.bg_bitmap[0] >> 8 == 0 &&
        //        self.bg_bitmap[1] >> 8 == 0 &&
        //        self.bg_palette[0] >> 8 == 0 &&
        //        self.bg_palette[1] >> 8 == 0);
        self.bg_bitmap[0] |= (PPU::reverse_byte(self.bg_bit_low) as u16) << 8;
        self.bg_bitmap[1] |= (PPU::reverse_byte(self.bg_bit_high) as u16) << 8;
        self.bg_palette[0] |= (self.bg_attr & 1) as u16 * 0xff00;
        self.bg_palette[1] |= ((self.bg_attr >> 1) & 1) as u16 * 0xff00;
    }

    #[inline(always)]
    fn shift_sprites(&mut self) {
        for (i, c) in self.sp_cnt.iter_mut().enumerate() {
            if self.sp_idx[i] > 0xff { break }
            let c0 = *c;
            match c0 {
                0 => {
                    let t = &mut self.sp_bitmap[i];
                    t[0] = t[0].wrapping_shr(1);
                    t[1] = t[1].wrapping_shr(1);
                },
                _ => *c = c0 - 1
            }
        }
    }

    #[inline(always)]
    fn shift_bgtile(&mut self, d: u8) {
        let t1 = &mut self.bg_bitmap;
        let t2 = &mut self.bg_palette;
        t1[0] = t1[0].wrapping_shr(d as u32);
        t1[1] = t1[1].wrapping_shr(d as u32);
        t2[0] = t2[0].wrapping_shr(d as u32);
        t2[1] = t2[1].wrapping_shr(d as u32);
    }

    #[inline(always)]
    fn wrapping_inc_cx(&mut self) {
        match self.v & 0x001f {
            31 => {
                self.v &= !0x001fu16; /* reset coarse x */
                self.v ^= 0x0400;     /* switch horizontal nametable */
            }
            _ => self.v += 1
        }
    }

    #[inline(always)]
    fn wrapping_inc_y(&mut self) {
        match (self.v & 0x7000) == 0x7000 {
            false => self.v += 0x1000, /* fine y < 7 */
            true => {
                self.v &= !0x7000u16;  /* fine y <- 0 */
                let y = match (self.v & 0x03e0) >> 5 {
                        29 => {self.v ^= 0x0800; 0}, /* at bottom of scanline */
                        31 => 0,                     /* do not switch nt */
                        y => y + 1
                    };
                self.v = (self.v & !0x03e0u16) | (y << 5);
            }
        }
    }

    #[inline(always)]
    fn reset_cx(&mut self) {
        self.v = (self.v & !0x041fu16) | (self.t & 0x041f);
    }

    #[inline(always)]
    fn reset_y(&mut self) {
        self.v = (self.v & !0x7be0u16) | (self.t & 0x7be0);
    }

    #[inline(always)]
    fn clear_sprite(&mut self) {
        self.oam2 = [0x100; 8];
    }

    fn eval_sprite(&mut self) {
        /* we use scanline here because s.y is the (actual y) - 1 */
        let mut nidx = 0;
        let mut n = 0;
        let scanline = self.scanline;
        let h = match self.get_spritesize() {
            0 => 8,
            _ => 16
        };
        for (i, s) in self.oam.iter().enumerate() {
            let y = s.y as u16;
            if y <= scanline && scanline < y + h {
                self.oam2[nidx] = i;
                nidx += 1;
                if nidx == 8 {
                    n = i + 1;
                    break;
                }
            }
        }
        if nidx == 8 {
            let mut m = 0;
            unsafe {
                let oam_raw = transmute::<&[Sprite; 64], &[[u8; 4]; 64]>(&self.oam);
                while n < 64 {
                    let y = oam_raw[n][m] as u16;
                    if y <= scanline && scanline < y + h {
                        self.ppustatus |= PPU::FLAG_OVERFLOW; /* set overflow */
                    } else {
                        m = (m + 1) & 3; /* emulates hardware bug */
                    }
                    n += 1;
                }
            }
        }
    }

    #[inline(always)]
    fn reverse_byte(mut x: u8) -> u8 {
        x = ((x & 0xaa) >> 1) | ((x & 0x55) << 1);
        x = ((x & 0xcc) >> 2) | ((x & 0x33) << 2);
        x = ((x & 0xf0) >> 4) | ((x & 0x0f) << 4);
        x
    }

    fn fetch_sprite(&mut self) {
        /* we use scanline here because s.y is the (actual y) - 1 */
        self.sp_idx = [0x100; 8];
        for (i, v) in self.oam2.iter().enumerate() {
            let j = *v;
            if j > 0xff { break }
            let s = &self.oam[j];
            let vflip = (s.attr & 0x80) == 0x80;
            let y0 = self.scanline - s.y as u16;
            let (ptable, tidx, y) = match self.get_spritesize() {
                0 => {
                    let y = if vflip {7 - y0 as u8} else {y0 as u8};
                    ((self.ppuctl as u16 & 0x08) << 9, s.tile, y)
                },
                _ => {
                    //assert!(false);
                    let y = if vflip {15 - y0 as u8} else {y0 as u8};
                    ((s.tile as u16 & 1) << 12,
                     (s.tile & !1u8) | (y >> 3),
                     y & 0x7)
                }
            };
            self.sp_idx[i] = j;
            self.sp_cnt[i] = s.x;
            let mut low = self.mem.read(ptable | ((tidx as u16) << 4) | 0x0 | y as u16);
            let mut high = self.mem.read(ptable | ((tidx as u16) << 4) | 0x8 | y as u16);
            if (s.attr & 0x40) != 0x40 {
                low = PPU::reverse_byte(low);
                high = PPU::reverse_byte(high);
            }
            self.sp_bitmap[i][0] = low;
            self.sp_bitmap[i][1] = high;
        }
    }

    #[inline(always)]
    fn get_bg_pidx(&self) -> u8 {
        if self.get_show_bg() {
            ((self.bg_bitmap[1] & 1) << 1) as u8 | (self.bg_bitmap[0] & 1) as u8
        } else { 0 }
    }

    #[inline(always)]
    fn get_sp_pidx(&self, i: usize) -> u8 {
        if self.get_show_sp() {
            ((self.sp_bitmap[i][1] & 1) << 1) | (self.sp_bitmap[i][0] & 1)
        } else { 0 }
    }

    fn render_pixel(&mut self) {
        //println!("ppuctl:{} ppumask:{}", self.ppuctl, self.ppumask);
        let x = self.cycle - 1;
        let bg_pidx =
            if x >= 8 || self.get_show_leftmost_bg() {self.get_bg_pidx()}
            else {0};
        let mut sp_pidx = 0x0;
        let mut sp_id = 0;
        let mut pri = 0x1;
        if x >= 8 || self.get_show_leftmost_sp() {
            for i in 0..8 {
                if self.sp_idx[i] > 0xff { break }
                let s = &self.oam[self.sp_idx[i]];
                if self.sp_cnt[i] != 0 { continue; } /* not active */
                match self.get_sp_pidx(i) {
                    0x0 => (),
                    pidx => {
                        if bg_pidx != 0 && self.sp_idx[i] == 0 && x != 0xff {
                            assert!(i == 0);
                            self.ppustatus |= PPU::FLAG_SPRITE_ZERO; /* set sprite zero hit */
                        }
                        sp_pidx = pidx;
                        sp_id = i;
                        pri = (s.attr >> 5) & 1;
                        break;
                    }
                }
            }
        }
        assert!(0 < self.cycle && self.cycle < 257);
        assert!(self.scanline < 240);
        self.scr.put((self.cycle - 1) as u8,
                     self.scanline as u8,
                     if (pri == 0 || bg_pidx == 0) && sp_pidx != 0 {
                        self.mem.read(0x3f10 |
                                      ((self.oam[self.sp_idx[sp_id]].attr & 3) << 2) as u16 |
                                      sp_pidx as u16)
                     } else {
                        self.mem.read(0x3f00 |
                                      (((self.bg_palette[1] & 1) << 3) |
                                       ((self.bg_palette[0] & 1) << 2)) as u16 |
                                      bg_pidx as u16)
                     });
    }

    pub fn new(mem: &'a VMem, scr: &'a Screen) -> Self {
        let ppuctl = 0x00;
        let ppumask = 0x00;
        let ppustatus = 0xa0;
        let oamaddr = 0x00;
        let w = false;
        let buffered_read = 0x00;
        let cycle = 370;
        let scanline = 240;
        PPU {
            scanline,
            ppuctl,
            ppumask,
            ppustatus,
            oamaddr,
            reg: 0,
            x: 0, v: 0, t: 0, w, cycle,
            bg_bitmap: [0; 2],
            bg_palette: [0; 2],
            bg_nt: 0, bg_attr: 0,
            bg_bit_low: 0, bg_bit_high: 0,
            oam: [Sprite{y: 0, tile: 0, attr: 0, x: 0}; 64],
            oam2: [0x100; 8],
            sp_idx: [0x100; 8],
            sp_bitmap: [[0; 2]; 8],
            sp_cnt: [0; 8],
            sp_zero_insight: false,
            rendering: false,
            buffered_read,
            mem, scr
        }
    }

    pub fn reset(&mut self) {
        self.ppuctl = 0x00;
        self.ppumask = 0x00;
        self.ppustatus = self.ppustatus & 0x80;
        self.w = false;
        self.buffered_read = 0x00;
        self.cycle = 370;
        self.scanline = 240;
    }

    pub fn tick(&mut self) -> bool {
        let cycle = self.cycle;
        if cycle == 0 {
            self.cycle = cycle + 1;
            return false;
        }
        let visible = self.scanline < 240;
        let pre_render = self.scanline == 261;
        self.rendering = pre_render || visible;
        if pre_render && cycle == 1 {
            /* clear vblank, sprite zero hit & overflow */
            self.ppustatus &= !(PPU::FLAG_VBLANK |
                                PPU::FLAG_SPRITE_ZERO | PPU::FLAG_OVERFLOW);

        } else if self.rendering && (self.get_show_bg() || self.get_show_sp()) {
            if pre_render && 279 < cycle && cycle < 305 {
                self.reset_y();
            } else {
                let shifting = 0 < cycle && cycle < 257; /* 1..256 */
                let fetch = shifting || (320 < cycle && cycle < 337);
                if fetch { /* 1..256 and 321..336 */
                    match cycle & 0x7 {
                        1 => {
                            if shifting && cycle > 1 {
                                self.load_bgtile();
                            }
                            self.fetch_nametable_byte();
                        },
                        3 => self.fetch_attrtable_byte(),
                        5 => self.fetch_low_bgtile_byte(),
                        7 => self.fetch_high_bgtile_byte(),
                        0 => self.wrapping_inc_cx(),
                        _ => ()
                    }
                    if visible {
                        match cycle {
                            1 => self.clear_sprite(), /* clear secondary OAM */
                            65 => self.eval_sprite(), /* sprite evaluation */
                            _ => ()
                        }
                    }
                    match cycle {
                        256 => {
                            self.wrapping_inc_y()
                        },
                        329 => self.load_bgtile(),
                        _ => ()
                    }
                } else if cycle > 336 { /* 337..340 */
                    if cycle & 1 == 1 {
                        if cycle == 337 {
                            self.shift_bgtile(8);
                            self.load_bgtile();
                        }
                        self.fetch_nametable_byte();
                    }
                } else if cycle == 257 {
                    /* we don't emulate fetch to per cycle precision because all data are fetched
                     * from the secondary OAM which is not subject to any change during this
                     * scanline */
                    self.reset_cx();
                    if visible {
                        self.fetch_sprite();
                    }
                }

                if shifting {
                    if visible {
                        self.render_pixel();
                    }
                    self.shift_bgtile(1);
                    self.shift_sprites();
                }
            }
        } else if self.scanline == 241 && cycle == 1 {
            self.scr.render();
            self.ppustatus |= PPU::FLAG_VBLANK;
            self.cycle += 1;
            return self.get_flag_nmi(); /* trigger cpu's NMI */
        }
        self.cycle += 1;
        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;
            if self.scanline > 261 {
                self.scanline = 0;
            }
        }
        false
    }
}
