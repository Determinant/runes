use core::cmp::min;
use core::mem::{size_of, transmute};

use crate::memory::{CPUBus, PPUMemory, VMem};
use crate::utils::{load_prefix, save_prefix, Read, Write};

pub trait Screen {
    fn put(&mut self, x: u8, y: u8, color: u8);
    fn render(&mut self);
    fn frame(&mut self);
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct Sprite {
    y: u8, /* is the (actualy y) - 1 */
    tile: u8,
    attr: u8,
    x: u8,
}

#[repr(C)]
pub struct PPU<'a> {
    /*-- begin state --*/
    pub scanline: u16,
    pub cycle: u16, /* cycle in the current scanline */
    /* registers */
    ppuctl: u8,
    ppumask: u8,
    ppustatus: u8,
    oamaddr: u8,
    reg: u8,
    x: u8,   /* fine x scroll */
    v: u16,  /* current vram addr */
    t: u16,  /* temporary vram addr */
    w: bool, /* first/second write toggle */
    f: bool, /* if it is an odd frame */
    /* rendering regs & latches */
    /* background register (current two tiles) */
    bg_pixel: u64,
    /* background latches for next tile */
    bg_nt: u8,
    bg_attr: u8,
    bg_bit_low: u8,
    bg_bit_high: u8,
    /* sprites */
    oam: [Sprite; 64],
    oam2: [u8; 8],
    sp_cache: [u16; 256], /* pre-computed sp value */
    vblank: bool,
    pub vblank_lines: bool,
    buffered_read: u8,
    early_read: bool,
    /*-- end state --*/

    /*-- begin sub-state --*/
    mem: PPUMemory<'a>,
    /*-- end sub-state --*/
    pub scr: &'a mut dyn Screen,
}

macro_rules! PPU_IGNORED_SIZE {
    () => {
        size_of::<PPUMemory>() + size_of::<&mut dyn Screen>()
    };
}

impl<'a> PPU<'a> {
    #[inline]
    pub fn write_ctl(&mut self, data: u8) {
        self.reg = data;
        self.ppuctl = data;
        self.t = (self.t & 0x73ff) | ((data as u16 & 3) << 10);
    }

    #[inline]
    pub fn write_mask(&mut self, data: u8) {
        self.reg = data;
        self.ppumask = data;
    }

    #[inline]
    pub fn read_status(&mut self) -> u8 {
        let res = (self.ppustatus & !0x1fu8) | (self.reg & 0x1f);
        self.ppustatus &= !PPU::FLAG_VBLANK;
        self.w = false;
        if self.scanline == 241 && self.cycle == 1 {
            self.early_read = true /* read before cycle 1 */
        }
        res
    }

    #[inline]
    pub fn write_oamaddr(&mut self, data: u8) {
        self.reg = data;
        self.oamaddr = data;
    }

    #[inline]
    pub fn write_oamdata(&mut self, data: u8) {
        self.reg = data;
        let addr = self.oamaddr as usize;
        self.get_oam_raw_mut()[addr] = data;
        self.oamaddr = self.oamaddr.wrapping_add(1);
    }

    #[inline]
    pub fn read_oamdata(&self) -> u8 {
        self.get_oam_raw()[self.oamaddr as usize]
    }

    #[inline]
    pub fn write_scroll(&mut self, data: u8) {
        self.reg = data;
        let data = data as u16;
        match self.w {
            false => {
                self.t = (self.t & 0x7fe0) | (data >> 3);
                self.x = (data & 0x07) as u8;
                self.w = true;
            }
            true => {
                self.t = (self.t & 0x0c1f) |
                    ((data & 0xf8) << 2) |
                    ((data & 0x07) << 12);
                self.w = false;
            }
        }
    }

    #[inline]
    pub fn write_addr(&mut self, data: u8) {
        self.reg = data;
        let data = data as u16;
        match self.w {
            false => {
                self.t = (self.t & 0x00ff) | ((data & 0x3f) << 8);
                self.w = true;
            }
            true => {
                self.t = (self.t & 0xff00) | data;
                self.v = self.t;
                self.w = false;
            }
        }
    }

    #[inline]
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
            _ => 32,
        });
        res
    }

    #[inline]
    pub fn write_data(&mut self, data: u8) {
        self.reg = data;
        self.mem.write(self.v, data);
        self.v = self.v.wrapping_add(match self.get_vram_inc() {
            0 => 1,
            _ => 32,
        });
    }

    #[inline]
    pub fn write_oamdma(&mut self, data: u8, bus: &CPUBus) {
        let cpu = bus.get_cpu();
        self.reg = data;
        let mut addr = (data as u16) << 8;
        let stall = 1 + (cpu.cycle & 1) + 512;
        bus.cpu_stall(stall);
        let mut oamaddr = self.oamaddr;
        for _ in 0..stall - 0x100 {
            cpu.mem.bus.tick()
        }
        {
            let oam_raw = self.get_oam_raw_mut();
            for _ in 0..0x100 {
                oam_raw[oamaddr as usize] = cpu.mem.read_without_tick(addr);
                addr = addr.wrapping_add(1);
                oamaddr = oamaddr.wrapping_add(1);
            }
        }
        self.oamaddr = oamaddr;
    }

    #[inline(always)]
    fn get_spritesize(&self) -> u8 {
        (self.ppuctl >> 5) & 1
    }
    #[inline(always)]
    pub fn get_flag_nmi(&self) -> bool {
        (self.ppuctl >> 7) == 1
    }
    #[inline(always)]
    fn get_vram_inc(&self) -> u8 {
        (self.ppuctl >> 2) & 1
    }
    #[inline(always)]
    fn get_show_leftmost_bg(&self) -> bool {
        (self.ppumask >> 1) & 1 == 1
    }
    #[inline(always)]
    fn get_show_leftmost_sp(&self) -> bool {
        (self.ppumask >> 2) & 1 == 1
    }
    #[inline(always)]
    pub fn get_show_bg(&self) -> bool {
        (self.ppumask >> 3) & 1 == 1
    }
    #[inline(always)]
    pub fn get_show_sp(&self) -> bool {
        (self.ppumask >> 4) & 1 == 1
    }
    #[inline(always)]
    fn get_flag_vblank(&self) -> bool {
        (self.ppustatus >> 7) & 1 == 1
    }
    #[inline(always)]
    fn get_oam_arr(&self) -> &[[u8; 4]; 64] {
        unsafe { transmute::<&[Sprite; 64], &[[u8; 4]; 64]>(&self.oam) }
    }
    #[inline(always)]
    fn get_oam_raw_mut(&mut self) -> &mut [u8; 256] {
        unsafe { transmute::<&mut [Sprite; 64], &mut [u8; 256]>(&mut self.oam) }
    }
    #[inline(always)]
    fn get_oam_raw(&self) -> &[u8; 256] {
        unsafe { transmute::<&[Sprite; 64], &[u8; 256]>(&self.oam) }
    }

    const FLAG_OVERFLOW: u8 = 1 << 5;
    const FLAG_SPRITE_ZERO: u8 = 1 << 6;
    const FLAG_VBLANK: u8 = 1 << 7;
    #[inline(always)]
    fn fetch_nametable_byte(&mut self) {
        self.bg_nt = self.mem.read_nametable(self.v & 0x0fff);
    }

    #[inline(always)]
    fn fetch_attrtable_byte(&mut self) {
        let v = self.v;
        /* the byte representing 4x4 tiles */
        let b = self.mem.read_nametable(
            0x03c0 | (v & 0x0c00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07),
        );
        self.bg_attr = (b >> ((v & 2) | ((v & 0x40) >> 4))) & 3;
    }

    #[inline(always)]
    fn fetch_low_bgtile_byte(&mut self) {
        /* 0x?000 */
        self.bg_bit_low = self.mem.read_mapper(
            ((self.ppuctl as u16 & 0x10) << 8) |
                                        /* 0x-??0 */
                                        ((self.bg_nt as u16) << 4) |
                                        /* 0x---? (0 - 7) */
                                        ((self.v >> 12) & 7) | 0x0,
        );
    }

    #[inline(always)]
    fn fetch_high_bgtile_byte(&mut self) {
        /* 0x?000 */
        self.bg_bit_high = self.mem.read_mapper(
            ((self.ppuctl as u16 & 0x10) << 8) |
                                        /* 0x-??0 */
                                        ((self.bg_nt as u16) << 4) |
                                        /* 0x---? (8 - f) */
                                        ((self.v >> 12) & 7) | 0x8,
        );
    }

    #[inline(always)]
    fn load_bgtile(&mut self) {
        /* load the tile bitmap to high 8 bits of bitmap,
         * assume the high 8 bits are zeros */
        debug_assert!(self.bg_pixel >> 32 == 0);
        let mut t: u64 = 0;
        let mut bl = self.bg_bit_low;
        let mut bh = self.bg_bit_high;
        for _ in 0..8 {
            t = (t << 4) |
                ((self.bg_attr << 2) | (bl & 1) | ((bh & 1) << 1)) as u64;
            bl >>= 1;
            bh >>= 1;
        }
        self.bg_pixel |= t << 32;
    }

    #[inline(always)]
    fn shift_bgtile(&mut self) {
        self.bg_pixel >>= 4;
    }

    #[inline(always)]
    fn wrapping_inc_cx(&mut self) {
        match self.v & 0x001f {
            31 => {
                self.v &= !0x001fu16; /* reset coarse x */
                self.v ^= 0x0400; /* switch horizontal nametable */
            }
            _ => self.v += 1,
        }
    }

    #[inline(always)]
    fn wrapping_inc_y(&mut self) {
        match (self.v & 0x7000) == 0x7000 {
            false => self.v += 0x1000, /* fine y < 7 */
            true => {
                self.v &= !0x7000u16; /* fine y <- 0 */
                let y = match (self.v & 0x03e0) >> 5 {
                    29 => {
                        self.v ^= 0x0800;
                        0
                    } /* at bottom of scanline */
                    31 => 0, /* do not switch nt */
                    y => y + 1,
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
        debug_assert!(self.scanline != 261);
        self.oam2 = [0xff; 8];
    }

    fn eval_sprite(&mut self) {
        debug_assert!(self.scanline != 261);
        /* we use scanline here because s.y is the (actual y) - 1 */
        let mut nidx = 0;
        let mut n = 0;
        let scanline = self.scanline;
        let h = match self.get_spritesize() {
            0 => 8,
            _ => 16,
        };
        for (i, s) in self.oam.iter().enumerate() {
            let y = s.y as u16;
            if y <= scanline && scanline < y + h {
                self.oam2[nidx] = i as u8;
                nidx += 1;
                if nidx == 8 {
                    n = i + 1;
                    break
                }
            }
        }
        if nidx == 8 {
            let mut m = 0;
            let mut ppustatus = self.ppustatus;
            {
                let oam_raw = self.get_oam_arr();
                while n < 64 {
                    let y = oam_raw[n][m] as u16;
                    if y <= scanline && scanline < y + h {
                        ppustatus |= PPU::FLAG_OVERFLOW; /* set overflow */
                    } else {
                        m = (m + 1) & 3; /* emulates hardware bug */
                    }
                    n += 1;
                }
            }
            self.ppustatus = ppustatus;
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
        if self.scanline == 261 {
            return
        }
        /* we use scanline here because s.y is the (actual y) - 1 */
        self.sp_cache = [0xffff; 256];
        for &j in self.oam2.iter() {
            if j == 0xff {
                break
            }
            let s = &self.oam[j as usize];
            let vflip = (s.attr & 0x80) == 0x80;
            let y0 = self.scanline - s.y as u16;
            let (ptable, tidx, y) = match self.get_spritesize() {
                0 => {
                    let y = if vflip { 7 - y0 as u8 } else { y0 as u8 };
                    ((self.ppuctl as u16 & 0x08) << 9, s.tile, y)
                }
                _ => {
                    let y = if vflip { 15 - y0 as u8 } else { y0 as u8 };
                    (
                        (s.tile as u16 & 1) << 12,
                        (s.tile & !1u8) | (y >> 3),
                        y & 0x7,
                    )
                }
            };
            let mut low = self
                .mem
                .read_mapper(ptable | ((tidx as u16) << 4) | 0x0 | y as u16);
            let mut high = self
                .mem
                .read_mapper(ptable | ((tidx as u16) << 4) | 0x8 | y as u16);
            if (s.attr & 0x40) == 0x40 {
                low = PPU::reverse_byte(low);
                high = PPU::reverse_byte(high);
            }
            let attr = s.attr & 3;
            let x_max = min(s.x as usize + 8, 256);
            /* pre-compute sprite pixels */
            for p in (&mut self.sp_cache[s.x as usize..x_max]).iter_mut().rev()
            {
                if *p == 0xffff {
                    let sp =
                        ((attr << 2) | ((high & 1) << 1) | (low & 1)) as u16;
                    if sp & 3 != 0x0 {
                        *p = ((if j == 0 {1} else {0}) << 15) | /* if zero sprite */
                             (((s.attr >> 5) as u16 & 1) << 8) | /* priority flag */
                             sp;
                    }
                }
                high >>= 1;
                low >>= 1;
            }
        }
    }

    fn render_pixel(&mut self) {
        let x = self.cycle - 1;
        let show_bg = self.get_show_bg();
        let show_sp = self.get_show_sp();

        let bg = if show_bg && (x >= 8 || self.get_show_leftmost_bg()) {
            (self.bg_pixel >> (self.x << 2)) & 0xf
        } else {
            0
        } as u16;
        let bg_pidx = bg & 3;
        let mut pri = 0x1;
        let mut sp = 0;
        let mut sp_pidx = 0x0;
        if show_sp && (x >= 8 || self.get_show_leftmost_sp()) {
            let p = self.sp_cache[x as usize];
            if p != 0xffff {
                if (p >> 15 == 1) && bg_pidx != 0 && x != 0xff {
                    self.ppustatus |= PPU::FLAG_SPRITE_ZERO; /* set sprite zero hit */
                }
                pri = (p >> 8) & 1;
                sp = p & 0x00ff;
                sp_pidx = sp & 3;
            }
        }
        debug_assert!(0 < self.cycle && self.cycle < 257);
        debug_assert!(self.scanline < 240);
        self.scr.put(
            (self.cycle - 1) as u8,
            self.scanline as u8,
            self.mem.read_palette(
                if (pri == 0 || bg_pidx == 0) && sp_pidx != 0 {
                    0x0010 | sp
                } else {
                    0x0000 |
                        match bg_pidx {
                            0 => 0,
                            _ => bg,
                        }
                },
            ) & 0x3f,
        );
    }

    pub fn new(mem: PPUMemory<'a>, scr: &'a mut dyn Screen) -> Self {
        let ppuctl = 0x00;
        let ppumask = 0x00;
        let ppustatus = 0xa0;
        let oamaddr = 0x00;
        let buffered_read = 0x00;
        let cycle = 0;
        let scanline = 241;
        PPU {
            scanline,
            ppuctl,
            ppumask,
            ppustatus,
            oamaddr,
            reg: 0,
            x: 0,
            v: 0,
            t: 0,
            w: false,
            f: true,
            cycle,
            bg_pixel: 0,
            bg_nt: 0,
            bg_attr: 0,
            bg_bit_low: 0,
            bg_bit_high: 0,
            oam: [Sprite {
                y: 0,
                tile: 0,
                attr: 0,
                x: 0,
            }; 64],
            oam2: [0xff; 8],
            sp_cache: [0xffff; 256],
            vblank: false,
            vblank_lines: true,
            buffered_read,
            early_read: false,
            mem,
            scr,
        }
    }

    pub fn load(&mut self, reader: &mut dyn Read) -> bool {
        load_prefix(self, PPU_IGNORED_SIZE!(), reader) && self.mem.load(reader)
    }

    pub fn save(&self, writer: &mut dyn Write) -> bool {
        save_prefix(self, PPU_IGNORED_SIZE!(), writer) && self.mem.save(writer)
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.ppuctl = 0x00;
        self.ppumask = 0x00;
        self.ppustatus = self.ppustatus & 0x80;
        self.w = false;
        self.buffered_read = 0x00;
        self.cycle = 0;
        self.scanline = 241;
        self.vblank_lines = true;
    }

    #[inline(always)]
    pub fn try_nmi(&mut self) -> bool {
        self.get_flag_vblank() && self.get_flag_nmi()
    }

    pub fn tick(&mut self, bus: &CPUBus) -> bool {
        let res = self._tick();
        self.mem.tick(bus);
        res
    }

    fn _tick(&mut self) -> bool {
        let cycle = self.cycle;
        if cycle == 0 {
            self.cycle = 1;
            if self.scanline == 240 {
                self.vblank_lines = true
            } else if self.scanline == 261 {
                self.vblank_lines = false
            }
            return false
        }
        let rendering = self.get_show_bg() || self.get_show_sp();
        let visible_line = self.scanline < 240;
        let pre_line = self.scanline == 261;
        if (pre_line || visible_line) && rendering {
            if pre_line && 279 < cycle && cycle < 305 {
                self.reset_y();
            } else {
                let visible_cycle = 0 < cycle && cycle < 257; /* 1..256 */
                let prefetch_cycle = 320 < cycle && cycle < 337;
                let fetch_cycle = visible_cycle || prefetch_cycle;
                if (visible_line && fetch_cycle) || (pre_line && prefetch_cycle)
                {
                    match cycle & 0x7 {
                        1 => {
                            self.load_bgtile();
                            self.fetch_nametable_byte();
                        }
                        3 => self.fetch_attrtable_byte(),
                        5 => self.fetch_low_bgtile_byte(),
                        7 => self.fetch_high_bgtile_byte(),
                        0 => self.wrapping_inc_cx(),
                        _ => (),
                    }
                    match cycle {
                        1 => self.clear_sprite(), /* clear secondary OAM */
                        65 => self.eval_sprite(), /* sprite evaluation */
                        256 => self.wrapping_inc_y(),
                        _ => (),
                    }
                    if visible_cycle {
                        self.render_pixel();
                    }
                    self.shift_bgtile();
                } else if cycle == 257 {
                    /* we don't emulate fetch to per cycle precision because all data are fetched
                     * from the secondary OAM which is not subject to any change during this
                     * scanline */
                    self.reset_cx();
                    self.fetch_sprite();
                    self.cycle = 258;
                    return false
                }
                /* skip at 338 because of 10-even_odd_timing test indicates an undocumented
                 * behavior of NES */
                if pre_line && cycle == 338 && self.f {
                    self.cycle = 340;
                    return false
                }
            }
        } else {
            if !rendering {
                self.bg_pixel = 0
            }
            if self.scanline == 241 && self.cycle == 1 {
                if !self.early_read {
                    self.ppustatus |= PPU::FLAG_VBLANK
                }
                self.early_read = false;
                self.vblank = true;
                self.scr.render();
                self.scr.frame();
                self.cycle = 2;
                return self.try_nmi()
            }
        }
        if pre_line && cycle == 1 {
            /* clear vblank, sprite zero hit & overflow */
            self.vblank = false;
            self.ppustatus &= !(PPU::FLAG_VBLANK |
                PPU::FLAG_SPRITE_ZERO |
                PPU::FLAG_OVERFLOW);
            self.bg_pixel = 0;
            self.cycle = 2;
            return false
        }
        self.cycle += 1;
        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;
            if self.scanline > 261 {
                self.scanline = 0;
                self.f = !self.f;
            }
        }
        false
    }
}
