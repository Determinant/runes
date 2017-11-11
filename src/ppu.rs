use memory::VMem;
use core::intrinsics::transmute;

pub trait Screen {
    fn put(&mut self, x: u8, y: u8, color: u8);
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

struct PPU<'a, 'b> {
    /* internal srams */
    nametable_ram: [u8; 2048],
    palette_ram: [u8; 32],
    scanline: u16,
    /* registers */
    ppuctl: u8,
    ppumask: u8,
    ppustatus: u8,
    
    x: u8, /* fine x scroll */
    v: u16, /* current vram addr */
    t: u16, /* temporary vram addr */
    w: bool, /* first/second write toggle */
    vblank: bool,
    cycle: u16, /* cycle in the current scanline */
    /* rendering regs & latches */
        /* background registers */
    bg_bitmap: [u16; 2],
    bg_palette: [u8; 2],
        /* background latches */
    bg_nt: u8,
    bg_attr: u8,
    bg_bit_low: u8,
    bg_bit_high: u8,
    /* sprites */
    oam: [Sprite; 64],
    oam2: [Sprite; 8],
    sp_bitmap: [[u8; 2]; 8],
    sp_cnt: [u8; 8],
    sp_zero_insight: bool,
    /* IO */
    mem: &'a mut VMem,
    scr: &'b mut Screen
}

impl<'a, 'b> PPU<'a, 'b> {
    #[inline(always)] fn get_spritesize(&self) -> u8 {(self.ppuctl >> 5) & 1}

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
                                        (self.v >> 12) | 0x0);
    }

    #[inline(always)]
    fn fetch_high_bgtile_byte(&mut self) {
                                        /* 0x?000 */
        self.bg_bit_high = self.mem.read(((self.ppuctl as u16 & 0x10) << 8) |
                                        /* 0x-??0 */
                                        ((self.bg_nt as u16) << 4) |
                                        /* 0x---? (8 - f) */
                                        (self.v >> 12) | 0x8);
    }

    fn load_bgtile(&mut self) {
        /* load the tile bitmap to high 8 bits of bitmap,
         * assume the high 8 bits are zeros */
        assert!(self.bg_bitmap[0] >> 8 == 0 &&
                self.bg_bitmap[1] >> 8 == 0);
        self.bg_bitmap[0] |= (self.bg_bit_low as u16) << 8;
        self.bg_bitmap[1] |= (self.bg_bit_high as u16) << 8;
        self.bg_palette[0] |= (self.bg_attr & 1) * 0xff;
        self.bg_palette[1] |= ((self.bg_attr >> 1) & 1) * 0xff;
    }

    #[inline(always)]
    fn shift_sprites(&mut self) {
        for (i, c) in self.sp_cnt.iter_mut().enumerate() {
            let c0 = *c;
            match c0 {
                0 => {
                    self.sp_bitmap[i][0] >>= 1;
                    self.sp_bitmap[i][1] >>= 1;
                },
                _ => *c = c0 - 1
            }
        }
    }

    #[inline(always)]
    fn shift_bgtile(&mut self, d: u8) {
        self.bg_bitmap[0] >>= d;
        self.bg_bitmap[1] >>= d;
        self.bg_palette[0] >>= d;
        self.bg_palette[1] >>= d;
    }

    fn wrapping_inc_cx(&mut self) {
        match self.v & 0x001f {
            31 => {
                self.v &= !0x001fu16; /* reset coarse x */
                self.v ^= 0x0400;     /* switch horizontal nametable */
            }
            _ => self.v += 1
        }
    }

    fn wrapping_inc_y(&mut self) {
        match (self.v & 0x7000) == 0x7000 {
            false => self.v += 0x1000, /* fine y < 7 */
            true => {
                self.v &= !0x7000u16;  /* fine y <- 0 */
                self.v = (self.v & !0x03e0u16) |
                    (match (self.v & 0x03e0) >> 5 {
                        29 => {self.v ^= 0x0800; 0}, /* at bottom of scanline */
                        31 => 0,                     /* do not switch nt */
                        y => y + 1
                    }) << 5;
            }
        }
    }

    #[inline(always)]
    fn reset_cx(&mut self) {
        self.v = (self.v & !0x001fu16) | (self.t & 0x001f);
    }

    #[inline(always)]
    fn reset_y(&mut self) {
        self.v = (self.v & !0x73e0u16) | (self.t & 0x73e0);
    }

    fn clear_sprite(&mut self) {
        self.oam2 = [Sprite{y: 0xff, tile: 0xff, attr: 0xff, x: 0xff}; 8];
    }

    fn eval_sprite(&mut self) {
        /* we use scanline here because s.y is the (actual y) - 1 */
        let mut nidx = 0;
        let mut n = 0;
        let h = match self.get_spritesize() {
            0 => 8,
            _ => 16
        };
        self.sp_zero_insight = false;
        for (i, s) in self.oam.iter().enumerate() {
            let y = s.y as u16;
            if y <= self.scanline && self.scanline < y + h {
                if nidx == 0 {
                    self.sp_zero_insight = true;
                }
                self.oam2[nidx] = *s;
                nidx += 1;
                if nidx == 8 {
                    n = i + 1;
                    break;
                }
            }
        }
        let mut m = 0;
        unsafe {
            let oam_raw = transmute::<[Sprite; 64], [[u8; 4]; 64]>(self.oam);
            while n < 64 {
                let y = oam_raw[n][m] as u16;
                if y <= self.scanline && self.scanline < y + h {
                    self.ppustatus |= 1 << 5; /* set overflow */
                } else {
                    m = (m + 1) & 3; /* emulates hardware bug */
                }
                n += 1;
            }
        }
    }

    fn reverse_byte(mut x: u8) -> u8 {
        x = ((x & 0xaa) >> 1) | ((x & 0x55) << 1);
        x = ((x & 0xcc) >> 2) | ((x & 0x33) << 2);
        x = ((x & 0xf0) >> 4) | ((x & 0x0f) << 4);
        x
    }

    fn fetch_sprite(&mut self) {
        /* we use scanline here because s.y is the (actual y) - 1 */
        for (i, v) in self.oam2.iter().enumerate() {
            let vflip = (v.attr & 0x80) == 0x80;
            let y0 = self.scanline - v.y as u16;
            let (ptable, tidx, y) = match self.get_spritesize() {
                0 => {
                    let y = if vflip {7 - y0 as u8} else {y0 as u8};
                    ((self.ppuctl as u16 & 0x08) << 9, v.tile, y)
                },
                _ => {
                    let y = if vflip {15 - y0 as u8} else {y0 as u8};
                    ((v.tile as u16 & 1) << 12,
                     (v.tile & !1u8) | (y >> 3),
                     y & 0x7)
                }
            };
            self.sp_cnt[i] = v.x;
            let mut low = self.mem.read(ptable | ((tidx as u16) << 4) | 0x0 | y as u16);
            let mut high = self.mem.read(ptable | ((tidx as u16) << 4) | 0x8 | y as u16);
            if (v.attr & 0x40) == 0x40 {
                low = PPU::reverse_byte(low);
                high = PPU::reverse_byte(high);
            }
            self.sp_bitmap[i][0] = low;
            self.sp_bitmap[i][1] = high;
        }
    }

    fn get_bgpixel(&self) -> u8 {
        let bg_idx = ((self.bg_bitmap[1] & 1) << 1) | (self.bg_bitmap[0] & 1);
        match bg_idx {
            0x0 => 0x0, /* transparent */
            _ => {
                let bg_pl = ((self.bg_palette[1] & 1) << 1) | (self.bg_palette[0] & 1);
                self.mem.read(0x3f00 | (bg_pl << 2) as u16 | bg_idx as u16)
            }
        }
    }

    fn get_sppixel(&self, idx: usize) -> u8 {
        let sp_idx = ((self.sp_bitmap[idx][1] & 1) << 1) | (self.sp_bitmap[idx][0] & 1);
        match sp_idx {
            0x0 => 0x0,
            _ => {
                let attr = self.oam2[idx].attr;
                self.mem.read(0x3f10 | ((attr & 3) << 2) as u16 | sp_idx as u16)
            }
        }
    }

    fn render_pixel(&mut self) {
        let bg = self.get_bgpixel();
        let mut sp = 0x0;
        let mut pri = 0x1;
        for i in 0..8 {
            if self.sp_cnt[i] != 0 { continue; } /* not active */
            match self.get_sppixel(i as usize) {
                0x0 => (),
                c => {
                    if self.sp_zero_insight && bg != 0 && i == 0 {
                        self.ppustatus |= 1 << 6; /* set sprite zero hit */
                    }
                    sp = c;
                    pri = (self.oam2[i].attr >> 5) & 1;
                    break;
                }
            }
        }
        assert!(0 < self.cycle && self.cycle < 257);
        assert!(self.scanline < 240);
        self.scr.put(self.cycle as u8 - 1,
                     self.scanline as u8,
                     if pri == 0 || bg == 0 { sp } else { bg });
    }

    pub fn tick(&mut self) -> bool {
        let cycle = self.cycle;
        if cycle == 0 {
            self.cycle = cycle + 1;
            return false;
        }
        let visible = self.scanline < 240;
        let pre_render = self.scanline == 261;
        let fill = pre_render || visible;
        if pre_render {
            if cycle == 1 {
                self.vblank = false;
                /* clear sprite zero hit & overflow */
                self.ppustatus &= !((1 << 6) | (1 << 5));
            } else if 279 < cycle && cycle < 305 {
                self.reset_y();
            }
        } 
        if fill {
            let shifting = 0 < cycle && cycle < 257; /* 1..256 */
            let fetch = shifting || (320 < cycle && cycle < 337);
            if fetch { /* 1..256 and 321..336 */
                match cycle & 0x7 {
                    1 => {
                        self.load_bgtile();
                        self.fetch_nametable_byte();
                    },
                    3 => self.fetch_attrtable_byte(),
                    5 => self.fetch_low_bgtile_byte(),
                    7 => self.fetch_high_bgtile_byte(),
                    0 => self.wrapping_inc_cx(),
                    _ => ()
                }
                if !pre_render {
                    match cycle {
                        1 => self.clear_sprite(), /* clear secondary OAM */
                        65 => self.eval_sprite(), /* sprite evaluation */
                        _ => ()
                    }
                }
                match cycle {
                    256 => self.wrapping_inc_y(),
                    328 => self.shift_bgtile(8),
                    _ => ()
                }
            } else if cycle > 336 { /* 337..340 */
                if cycle & 1 == 1 {
                    self.fetch_nametable_byte();
                }
            } else { /* 257..320 */
                if cycle == 257 {
                    /* we don't emulate fetch to per cycle precision because all data are fetched
                     * from the secondary OAM which is not subject to any change during this
                     * scanline */
                    self.reset_cx();
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
        } else if self.scanline == 241 && cycle == 1 {
            self.vblank = true;
            self.cycle += 1;
            return true /* trigger cpu's NMI */
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
