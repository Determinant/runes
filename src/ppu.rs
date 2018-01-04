#![allow(dead_code)]
use memory::{VMem, PPUMemory, CPUBus};
use core::intrinsics::transmute;

pub trait Screen {
    fn put(&mut self, x: u8, y: u8, color: u8);
    fn render(&mut self);
    fn frame(&mut self);
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
    pub scanline: u16,
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
    f: bool, /* if it is an odd frame */
    pub cycle: u16, /* cycle in the current scanline */
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
    oam2: [usize; 8],
    sp_pixel: [u32; 8],
    sp_idx: [usize; 8],
    sp_cnt: [u8; 8],
    vblank: bool,
    pub vblank_lines: bool,
    buffered_read: u8,
    early_read: bool,
    /* IO */
    mem: PPUMemory<'a>,
    pub scr: &'a mut Screen,
    rendering: bool,
    cb_matrix: [[fn (&mut PPU<'a>) -> bool; 341]; 262],
    //pub elapsed: u32,
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
        self.rendering = self.get_show_bg() || self.get_show_sp();
        if !self.rendering {
            self.bg_pixel = 0
        }
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
        self.get_oam_raw_mut()[self.oamaddr as usize] = data;
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
            },
            true => {
                self.t = (self.t & 0x0c1f) | ((data & 0xf8) << 2) | ((data & 0x07) << 12);
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
            },
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
            _ => 32
        });
        res
    }

    #[inline]
    pub fn write_data(&mut self, data: u8) {
        self.reg = data;
        self.mem.write(self.v, data);
        self.v = self.v.wrapping_add(match self.get_vram_inc() {
            0 => 1,
            _ => 32
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

    #[inline(always)] fn get_spritesize(&self) -> u8 {(self.ppuctl >> 5) & 1}
    #[inline(always)] pub fn get_flag_nmi(&self) -> bool { (self.ppuctl >> 7) == 1 }
    #[inline(always)] fn get_vram_inc(&self) -> u8 { (self.ppuctl >> 2) & 1}
    #[inline(always)] fn get_show_leftmost_bg(&self) -> bool { (self.ppumask >> 1) & 1 == 1}
    #[inline(always)] fn get_show_leftmost_sp(&self) -> bool { (self.ppumask >> 2) & 1 == 1}
    #[inline(always)] pub fn get_show_bg(&self) -> bool { (self.ppumask >> 3) & 1 == 1}
    #[inline(always)] pub fn get_show_sp(&self) -> bool { (self.ppumask >> 4) & 1 == 1}
    #[inline(always)] pub fn get_flag_vblank(&self) -> bool { (self.ppustatus >> 7) & 1 == 1 }
    #[inline(always)] fn get_oam_arr(&self) -> &[[u8; 4]; 64] {
        unsafe {transmute::<&[Sprite; 64], &[[u8; 4]; 64]>(&self.oam)}
    }
    #[inline(always)] fn get_oam_raw_mut(&mut self) -> &mut[u8; 256] {
        unsafe {transmute::<&mut[Sprite; 64], &mut[u8; 256]>(&mut self.oam)}
    }
    #[inline(always)] fn get_oam_raw(&self) -> &[u8; 256] {
        unsafe {transmute::<&[Sprite; 64], &[u8; 256]>(&self.oam)}
    }

    const FLAG_OVERFLOW: u8 = 1 << 5;
    const FLAG_SPRITE_ZERO: u8 = 1 << 6;
    const FLAG_VBLANK: u8 = 1 << 7;
    #[inline(always)]
    fn fetch_nametable_byte(ppu: &mut PPU) {
        ppu.bg_nt = ppu.mem.read_nametable(ppu.v & 0x0fff)
    }

    #[inline(always)]
    fn fetch_attrtable_byte(ppu: &mut PPU) {
        let v = ppu.v;
        /* the byte representing 4x4 tiles */
        let b = ppu.mem.read_nametable(0x03c0 | (v & 0x0c00) |
                            ((v >> 4) & 0x38) | ((v >> 2) & 0x07));
        ppu.bg_attr = (b >> ((v & 2) | ((v & 0x40) >> 4))) & 3;
    }

    #[inline(always)]
    fn fetch_low_bgtile_byte(ppu: &mut PPU) {
                                        /* 0x?000 */
        ppu.bg_bit_low = ppu.mem.read_mapper(((ppu.ppuctl as u16 & 0x10) << 8) |
                                        /* 0x-??0 */
                                        ((ppu.bg_nt as u16) << 4) |
                                        /* 0x---? (0 - 7) */
                                        ((ppu.v >> 12) & 7) | 0x0)
    }

    #[inline(always)]
    fn fetch_high_bgtile_byte(ppu: &mut PPU) {
                                        /* 0x?000 */
        ppu.bg_bit_high = ppu.mem.read_mapper(((ppu.ppuctl as u16 & 0x10) << 8) |
                                        /* 0x-??0 */
                                        ((ppu.bg_nt as u16) << 4) |
                                        /* 0x---? (8 - f) */
                                        ((ppu.v >> 12) & 7) | 0x8)
    }

    #[inline(always)]
    fn load_bgtile(ppu: &mut PPU) {
        /* load the tile bitmap to high 8 bits of bitmap,
         * assume the high 8 bits are zeros */
        debug_assert!(ppu.bg_pixel >> 32 == 0);
        let mut t: u64 = 0;
        let mut bl = ppu.bg_bit_low;
        let mut bh = ppu.bg_bit_high;
        for _ in 0..8 {
            t = (t << 4) | ((ppu.bg_attr << 2) | (bl & 1) | ((bh & 1) << 1)) as u64;
            bl >>= 1;
            bh >>= 1;
        }
        ppu.bg_pixel |= t << 32;
    }

    fn nop(ppu: &mut PPU) -> bool {
        false
    }

    fn visible_8_1(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::load_bgtile(ppu);
            PPU::fetch_nametable_byte(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn visible_8_3(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::fetch_attrtable_byte(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn visible_8_5(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::fetch_low_bgtile_byte(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn visible_8_7(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::fetch_high_bgtile_byte(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn visible_8_0(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::wrapping_inc_cx(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn visible_1(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::visible_8_1(ppu);
            PPU::clear_sprite(ppu);
        }
        false
    }

    fn visible_65(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::visible_8_1(ppu);
            PPU::eval_sprite(ppu);
        }
        false
    }

    fn visible_256(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::visible_8_0(ppu);
            PPU::wrapping_inc_y(ppu);
        }
        false
    }

    fn visible_257(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::reset_cx(ppu);
            PPU::fetch_sprite(ppu);
        }
        false
    }

    fn rendering_8_1(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::load_bgtile(ppu);
            PPU::fetch_nametable_byte(ppu);
            PPU::render_pixel(ppu);
            PPU::shift_sprites(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn rendering_8_3(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::fetch_attrtable_byte(ppu);
            PPU::render_pixel(ppu);
            PPU::shift_sprites(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn rendering_8_5(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::fetch_low_bgtile_byte(ppu);
            PPU::render_pixel(ppu);
            PPU::shift_sprites(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn rendering_8_7(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::fetch_high_bgtile_byte(ppu);
            PPU::render_pixel(ppu);
            PPU::shift_sprites(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn rendering_8_0(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::wrapping_inc_cx(ppu);
            PPU::render_pixel(ppu);
            PPU::shift_sprites(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn rendering_1(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::rendering_8_1(ppu);
            PPU::clear_sprite(ppu);
        }
        false
    }

    fn rendering_65(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::rendering_8_1(ppu);
            PPU::eval_sprite(ppu);
        }
        false
    }

    fn rendering_256(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::rendering_8_0(ppu);
            PPU::wrapping_inc_y(ppu);
        }
        false
    }

    fn visible_8_other(ppu: &mut PPU) -> bool{
        if ppu.rendering {
            PPU::shift_bgtile(ppu)
        }
        false
    }

    fn rendering_8_other(ppu: &mut PPU) -> bool {
        if ppu.rendering {
            PPU::render_pixel(ppu);
            PPU::shift_sprites(ppu);
            PPU::shift_bgtile(ppu);
        }
        false
    }

    fn skip_cycle(ppu: &mut PPU) -> bool {
        if ppu.f {
            ppu.cycle += 1
        }
        false
    }

    fn vblank_cycle(ppu: &mut PPU) -> bool {
        if !ppu.early_read {
            ppu.ppustatus |= PPU::FLAG_VBLANK
        }
        //self.elapsed = 0;
        //println!("vbl");
        ppu.early_read = false;
        ppu.vblank = true;
        ppu.scr.render();
        ppu.try_nmi()
    }

    fn vblank_clear_cycle(ppu: &mut PPU) -> bool {
        /* clear vblank, sprite zero hit & overflow */
        ppu.vblank = false;
        ppu.ppustatus &= !(PPU::FLAG_VBLANK |
                            PPU::FLAG_SPRITE_ZERO | PPU::FLAG_OVERFLOW);
        ppu.bg_pixel = 0;
        false
    }
    
    fn zero_cycle(ppu: &mut PPU) -> bool {
        if ppu.scanline == 240 {
            ppu.vblank_lines = true
        } else if ppu.scanline == 261 {
            ppu.vblank_lines = false
        }
        false
    }

    #[inline(always)]
    fn shift_sprites(ppu: &mut PPU) {
        for (i, c) in ppu.sp_cnt.iter_mut().enumerate() {
            if ppu.sp_idx[i] > 0xff { break }
            let c0 = *c;
            match c0 {
                0 => ppu.sp_pixel[i] >>= 4,
                _ => *c = c0 - 1
            }
        }
    }

    #[inline(always)]
    fn shift_bgtile(&mut self) {
        self.bg_pixel >>= 4;
    }

    #[inline(always)]
    fn wrapping_inc_cx(ppu: &mut PPU) {
        match ppu.v & 0x001f {
            31 => {
                ppu.v &= !0x001fu16; /* reset coarse x */
                ppu.v ^= 0x0400;     /* switch horizontal nametable */
            }
            _ => ppu.v += 1
        }
    }

    #[inline(always)]
    fn wrapping_inc_y(ppu: &mut PPU) {
        match (ppu.v & 0x7000) == 0x7000 {
            false => ppu.v += 0x1000, /* fine y < 7 */
            true => {
                ppu.v &= !0x7000u16;  /* fine y <- 0 */
                let y = match (ppu.v & 0x03e0) >> 5 {
                        29 => {ppu.v ^= 0x0800; 0}, /* at bottom of scanline */
                        31 => 0,                     /* do not switch nt */
                        y => y + 1
                    };
                ppu.v = (ppu.v & !0x03e0u16) | (y << 5);
            }
        }
    }

    #[inline(always)]
    fn reset_cx(&mut self) {
        self.v = (self.v & !0x041fu16) | (self.t & 0x041f);
    }

    #[inline(always)]
    fn reset_y(ppu: &mut PPU) {
        ppu.v = (ppu.v & !0x7be0u16) | (ppu.t & 0x7be0)
    }

    fn reset_y_cycle(ppu: &mut PPU) -> bool {
        PPU::reset_y(ppu);
        false
    }

    #[inline(always)]
    fn clear_sprite(ppu: &mut PPU) {
        debug_assert!(ppu.scanline != 261);
        ppu.oam2 = [0x100; 8];
    }

    #[inline(always)]
    fn eval_sprite(ppu: &mut PPU) {
        debug_assert!(ppu.scanline != 261);
        /* we use scanline here because s.y is the (actual y) - 1 */
        let mut nidx = 0;
        let mut n = 0;
        let scanline = ppu.scanline;
        let h = match ppu.get_spritesize() {
            0 => 8,
            _ => 16
        };
        for (i, s) in ppu.oam.iter().enumerate() {
            let y = s.y as u16;
            if y <= scanline && scanline < y + h {
                ppu.oam2[nidx] = i;
                nidx += 1;
                if nidx == 8 {
                    n = i + 1;
                    break;
                }
            }
        }
        if nidx == 8 {
            let mut m = 0;
            let mut ppustatus = ppu.ppustatus;
            {
                let oam_raw = ppu.get_oam_arr();
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
            ppu.ppustatus = ppustatus;
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
        if self.scanline == 261 { return }
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
                    let y = if vflip {15 - y0 as u8} else {y0 as u8};
                    ((s.tile as u16 & 1) << 12,
                     (s.tile & !1u8) | (y >> 3),
                     y & 0x7)
                }
            };
            self.sp_idx[i] = j;
            self.sp_cnt[i] = s.x;
            let mut low = self.mem.read_mapper(ptable | ((tidx as u16) << 4) | 0x0 | y as u16);
            let mut high = self.mem.read_mapper(ptable | ((tidx as u16) << 4) | 0x8 | y as u16);
            if (s.attr & 0x40) == 0x40 {
                low = PPU::reverse_byte(low);
                high = PPU::reverse_byte(high);
            }
            let attr = s.attr & 3;
            let mut t = 0u32;
            for _ in 0..8 {
                t = (t << 4) | ((attr << 2) | ((high & 1) << 1) | (low & 1)) as u32;
                high >>= 1;
                low >>= 1;
            }
            self.sp_pixel[i] = t;
        }
    }

    fn render_pixel(ppu: &mut PPU) {
        let x = ppu.cycle - 1;
        let bg = ((ppu.bg_pixel >> (ppu.x << 2)) & 0xf) as u16;
        let bg_pidx =
            if x >= 8 || ppu.get_show_leftmost_bg() {
                if ppu.get_show_bg() {bg & 3} else {0}
            } else {0};
        let mut sp_pidx = 0x0;
        let mut pri = 0x1;
        let mut sp = 0;
        let show_sp = ppu.get_show_sp();
        if x >= 8 || ppu.get_show_leftmost_sp() {
            for i in 0..8 {
                if ppu.sp_idx[i] > 0xff { break }
                if ppu.sp_cnt[i] != 0 { continue; } /* not active */
                let s = &ppu.oam[ppu.sp_idx[i]];
                sp = if show_sp {(ppu.sp_pixel[i] & 0xf) as u16} else { 0 };
                match sp & 3 {
                    0x0 => (),
                    pidx => {
                        if bg_pidx != 0 && ppu.sp_idx[i] == 0 &&
                           x != 0xff && s.y != 0xff {
                            ppu.ppustatus |= PPU::FLAG_SPRITE_ZERO; /* set sprite zero hit */
                        }
                        sp_pidx = pidx;
                        pri = (s.attr >> 5) & 1;
                        break;
                    }
                }
            }
        }
        debug_assert!(0 < ppu.cycle && ppu.cycle < 257);
        debug_assert!(ppu.scanline < 240);
        ppu.scr.put((ppu.cycle - 1) as u8,
                     ppu.scanline as u8,
                     ppu.mem.read_palette(if (pri == 0 || bg_pidx == 0) && sp_pidx != 0 {
                        0x0010 | sp
                     } else {
                        0x0000 | match bg_pidx {
                            0 => 0,
                            _ => bg
                        }
                     }) & 0x3f);
    }

    pub fn new(mem: PPUMemory<'a>, scr: &'a mut Screen) -> Self {
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
            x: 0, v: 0, t: 0, w: false, f: true, cycle,
            bg_pixel: 0,
            bg_nt: 0, bg_attr: 0,
            bg_bit_low: 0, bg_bit_high: 0,
            oam: [Sprite{y: 0, tile: 0, attr: 0, x: 0}; 64],
            oam2: [0x100; 8],
            sp_idx: [0x100; 8],
            sp_pixel: [0; 8],
            sp_cnt: [0; 8],
            vblank: false,
            vblank_lines: true,
            buffered_read,
            early_read: false,
            mem, scr,
            rendering: false,
            cb_matrix: [[PPU::nop; 341]; 262]
            //elapsed: 0,
        }
    }

    pub fn reset(&mut self) {
        self.ppuctl = 0x00;
        self.ppumask = 0x00;
        self.ppustatus = self.ppustatus & 0x80;
        self.w = false;
        self.buffered_read = 0x00;
        self.cycle = 0;
        self.scanline = 241;
        self.vblank_lines = true;
        self.rendering = false;
    }

    #[inline(always)]
    pub fn try_nmi(&mut self) -> bool {
        self.get_flag_vblank() && self.get_flag_nmi()
    }

    pub fn tick(&mut self, bus: &CPUBus) -> bool {
        let scanline = self.scanline as usize;
        let cycle = self.cycle as usize;
        let res = self.cb_matrix[scanline][cycle](self);
        self.tick_x_y();
        self.mem.tick(bus);
        res
    }

    fn tick_x_y(&mut self) {
        self.cycle += 1;
        if self.cycle > 340 {
            self.cycle = 0;
            self.scanline += 1;
            if self.scanline > 261 {
                self.scanline = 0;
                self.f = !self.f;
            }
        }
    }

    pub fn powerup(&mut self) {
        for _ in 0..262 {
            for _ in 0..341 {
                self.fill_cb_matrix_tick();
                self.tick_x_y();
            }
        }
    }

    fn fill_cb_matrix_tick(&mut self) {
        //self.elapsed += 1;
        let cycle = self.cycle;
        let f = &mut self.cb_matrix[self.scanline as usize][self.cycle as usize];
        if cycle == 0 {
            *f = PPU::zero_cycle;
            return
        }
        let visible_line = self.scanline < 240;
        let pre_line = self.scanline == 261;
        if pre_line || visible_line {
            if pre_line && 279 < cycle && cycle < 305 {
                *f = PPU::reset_y_cycle
            } else {
                let visible_cycle = 0 < cycle && cycle < 257; /* 1..256 */
                let prefetch_cycle = 320 < cycle && cycle < 337;
                if (visible_line && prefetch_cycle) || (pre_line && prefetch_cycle) {
                    *f = match cycle {
                        1 => PPU::visible_1,
                        65 => PPU::visible_65,
                        256 => PPU::visible_256,
                        _ => match cycle & 0x7 {
                            0 => PPU::visible_8_1,
                            1 => PPU::visible_8_1,
                            3 => PPU::visible_8_3,
                            5 => PPU::visible_8_5,
                            7 => PPU::visible_8_7,
                            _ => PPU::visible_8_other
                        }
                    }
                } else if visible_line && visible_cycle {
                    *f = match cycle {
                        1 => PPU::rendering_1,
                        65 => PPU::rendering_65,
                        256 => PPU::rendering_256,
                        _ => match cycle & 0x7 {
                            0 => PPU::rendering_8_1,
                            1 => PPU::rendering_8_1,
                            3 => PPU::rendering_8_3,
                            5 => PPU::rendering_8_5,
                            7 => PPU::rendering_8_7,
                            _ => PPU::rendering_8_other
                        }
                    }
                } else if cycle == 257 {
                    *f = PPU::visible_257
                }
                /* skip at 338 because of 10-even_odd_timing test indicates an undocumented
                 * behavior of NES */
                if pre_line {
                    if cycle == 338 {
                        *f = PPU::skip_cycle
                    } else if cycle == 1 {
                        *f = PPU::vblank_clear_cycle
                    }
                }
            }
        } else {
            *f = if self.scanline == 241 && self.cycle == 1 {
                PPU::vblank_cycle
            } else {
                PPU::nop
            }
        }
    }
}
