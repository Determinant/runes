#![allow(dead_code)]
macro_rules! make_optable {
    ($x:ident, $t: ty) => (pub const $x: [$t; 0x100] = [
    /*  0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf */
        brk, ora, nil, nil, nil, ora, asl, nil, php, ora, asl, nil, nil, ora, asl, nil,
        bpl, ora, nil, nil, nil, ora, asl, nil, clc, ora, nil, nil, nil, ora, asl, nil,
        jsr, and, nil, nil, bit, and, rol, nil, plp, and, rol, nil, bit, and, rol, nil,
        bmi, and, nil, nil, nil, and, rol, nil, sec, and, nil, nil, nil, and, rol, nil,
        rti, eor, nil, nil, nil, eor, lsr, nil, pha, eor, lsr, nil, jmp, eor, lsr, nil,
        bvc, eor, nil, nil, nil, eor, lsr, nil, cli, eor, nil, nil, nil, eor, lsr, nil,
        rts, adc, nil, nil, nil, adc, ror, nil, pla, adc, ror, nil, jmp, adc, ror, nil,
        bvs, adc, nil, nil, nil, adc, ror, nil, sei, adc, nil, nil, nil, adc, ror, nil,
        nil, sta, nil, nil, sty, sta, stx, nil, dey, nil, txa, nil, sty, sta, stx, nil,
        bcc, sta, nil, nil, sty, sta, stx, nil, tya, sta, txs, nil, nil, sta, nil, nil,
        ldy, lda, ldx, nil, ldy, lda, ldx, nil, tay, lda, tax, nil, ldy, lda, ldx, nil,
        bcs, lda, nil, nil, ldy, lda, ldx, nil, clv, lda, tsx, nil, ldy, lda, ldx, nil,
        cpy, cmp, nil, nil, cpy, cmp, dec, nil, iny, cmp, dex, nil, cpy, cmp, dec, nil,
        bne, cmp, nil, nil, nil, cmp, dec, nil, cld, cmp, nil, nil, nil, cmp, dec, nil,
        cpx, sbc, nil, nil, cpx, sbc, inc, nil, inx, sbc, nop, nil, cpx, sbc, inc, nil,
        beq, sbc, nil, nil, nil, sbc, inc, nil, sed, sbc, nil, nil, nil, sbc, inc, nil
    ];);
}

macro_rules! make_addrtable {
    ($x:ident, $t: ty) => (pub const $x: [$t; 0x100] = [
       nil, xin, nil, nil, nil, zpg, zpg, nil, nil, imm, acc, nil, nil, abs, abs, nil,
       rel, iny, nil, nil, nil, zpx, zpx, nil, nil, aby, nil, nil, nil, abx, abx, nil,
       abs, xin, nil, nil, zpg, zpg, zpg, nil, nil, imm, acc, nil, abs, abs, abs, nil,
       rel, iny, nil, nil, nil, zpx, zpx, nil, nil, aby, nil, nil, nil, abx, abx, nil,
       nil, xin, nil, nil, nil, zpg, zpg, nil, nil, imm, acc, nil, abs, abs, abs, nil,
       rel, iny, nil, nil, nil, zpx, zpx, nil, nil, aby, nil, nil, nil, abx, abx, nil,
       nil, xin, nil, nil, nil, zpg, zpg, nil, nil, imm, acc, nil, ind, abs, abs, nil,
       rel, iny, nil, nil, nil, zpx, zpx, nil, nil, aby, nil, nil, nil, abx, abx, nil,
       nil, xin, nil, nil, zpg, zpg, zpg, nil, nil, nil, nil, nil, abs, abs, abs, nil,
       rel, iny, nil, nil, zpx, zpx, zpy, nil, nil, aby, nil, nil, nil, abx, nil, nil,
       imm, xin, imm, nil, zpg, zpg, zpg, nil, nil, imm, nil, nil, abs, abs, abs, nil,
       rel, iny, nil, nil, zpx, zpx, zpy, nil, nil, aby, nil, nil, abx, abx, aby, nil,
       imm, xin, nil, nil, zpg, zpg, zpg, nil, nil, imm, nil, nil, abs, abs, abs, nil,
       rel, iny, nil, nil, nil, zpx, zpx, nil, nil, aby, nil, nil, nil, abx, abx, nil,
       imm, xin, nil, nil, zpg, zpg, zpg, nil, nil, imm, nil, nil, abs, abs, abs, nil,
       rel, iny, nil, nil, nil, zpx, zpx, nil, nil, aby, nil, nil, nil, abx, abx, nil
    ];);
}

macro_rules! ids2strs {
    ($($x:ident), *) => {
        $(#[allow(non_upper_case_globals)]
            const $x: &str = stringify!($x);)*
    };
}

pub mod disasm {
    mod disops {
        make_optable!(OPS, &str);
        ids2strs!(adc, and, asl, bcc, bcs, beq, bit, bmi,
                  bne, bpl, brk, bvc, bvs, clc, cld, cli,
                  clv, cmp, cpx, cpy, dec, dex, dey, eor,
                  inc, inx, iny, jmp, jsr, lda, ldx, ldy,
                  lsr, nop, ora, pha, php, pla, plp, rol,
                  ror, rti, rts, sbc, sec, sed, sei, sta,
                  stx, sty, tax, tay, tsx, txa, txs, tya, nil);
    }
    
    mod disaddr {
        pub type T<'a, 'b> = &'a mut Iterator<Item=&'b u8>;
        make_addrtable!(ADDR_MODES, fn (T) -> String);
        fn acc(_code: T) -> String {
            "a".to_string()
        }
        fn imm(code: T) -> String {
            format!("#${:02x}", code.next().unwrap())
        }
        fn zpg(code: T) -> String {
            format!("${:02x}", code.next().unwrap())
        }
        fn zpx(code: T) -> String {
            format!("${:02x}, x", code.next().unwrap())
        }
        fn zpy(code: T) -> String {
            format!("${:02x}, y", code.next().unwrap())
        }
        fn rel(code: T) -> String {
            let b = *code.next().unwrap() as i8;
            if b >= 0 {
                format!("+${:02x}, x", b)
            } else {
                format!("-${:02x}, x", -b)
            }
        }
        fn abs(code: T) -> String {
            let low = *code.next().unwrap() as u16;
            let high = *code.next().unwrap() as u16;
            format!("${:04x}", (high << 8) | low)
        }
        fn abx(code: T) -> String {
            let low = *code.next().unwrap() as u16;
            let high = *code.next().unwrap() as u16;
            format!("${:04x}, x", (high << 8) | low)
        }
        fn aby(code: T) -> String {
            let low = *code.next().unwrap() as u16;
            let high = *code.next().unwrap() as u16;
            format!("${:04x}, y", (high << 8) | low)
        }
        fn ind(code: T) -> String {
            let low = *code.next().unwrap() as u16;
            let high = *code.next().unwrap() as u16;
            format!("(${:04x})", (high << 8) | low)
        }
        fn xin(code: T) -> String {
            format!("(${:02x}, x)", code.next().unwrap())
        }
        fn iny(code: T) -> String {
            format!("(${:02x}), y", code.next().unwrap())
        }
        fn nil(_code: T) -> String {
            "".to_string()
        }
    }

    pub struct Disassembler<'a, T> where T: Iterator<Item=&'a u8> {
        raw_code: T
    }
    
    impl<'a, T> Disassembler<'a, T> where T: Iterator<Item=&'a u8> {
        pub fn new(raw_code: T) -> Self {
            Disassembler{raw_code}
        }
        fn parse(opcode: u8, code: &mut T) -> String {
            format!("{} {}", disops::OPS[opcode as usize],
                             disaddr::ADDR_MODES[opcode as usize](code))
        }
    }
    
    impl<'a, T> Iterator for Disassembler<'a, T> where T: Iterator<Item=&'a u8> {
        type Item = String;
        fn next(&mut self) -> Option<Self::Item> {
            match self.raw_code.next() {
                Some(opcode) => Some(Disassembler::parse(*opcode, &mut self.raw_code)),
                None => None
            }
        }
    }
    
    pub fn parse(opcode: u8, code: &[u8]) -> String {
        Disassembler::parse(opcode, &mut code.iter())
    }
}

macro_rules! make16 {
    ($high: expr, $low: expr) => ((($high as u16) << 8) | ($low as u16));
}

macro_rules! read16 {
    ($mem: expr, $laddr: expr) => (make16!($mem.read($laddr.wrapping_add(1)),
                                                $mem.read($laddr)));
}

mod ops {
    use mos6502::{CPU, AddrMode};
    make_optable!(OPS, fn (&mut CPU));

    const CARRY_FLAG: u8 = 1 << 0;
    const ZERO_FLAG: u8 = 1 << 1;
    const INT_FLAG: u8 = 1 << 2;
    const DEC_FLAG: u8 = 1 << 3;
    const BRK_FLAG: u8 = 1 << 4;
    const OVER_FLAG: u8 = 1 << 6;
    const NEG_FLAG: u8 = 1 << 7;

    macro_rules! check_zero {
        ($st: ident, $r: expr) => ($st |= (($r as u8 == 0) as u8) << 1);
    }
    macro_rules! check_neg {
        ($st: ident, $r: expr) => ($st |= ($r as u8 & NEG_FLAG) as u8);
    }

    fn adc(cpu: &mut CPU) {
        let opr1 = cpu.a as u16;
        let opr2 = match cpu.addr_mode {
                    AddrMode::Immediate => cpu.imm_val,
                    AddrMode::Accumulator => cpu.a,
                    AddrMode::EffAddr => cpu.mem.read(cpu.ea)
                    } as u16;
        let res = opr1 + opr2 + cpu.get_carry() as u16;
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | OVER_FLAG | NEG_FLAG);
        cpu.a = res as u8;
        status |= (res > 0xff) as u8; /* carry flag */
        check_zero!(status, res);
        status |= ((((opr1 ^ opr2) as u8 & 0x80) ^ 0x80) &
                     ((opr1 ^ res) as u8 & 0x80)) >> 1; /* over flag */
        check_neg!(status, res);
        cpu.status = status;
    }


    fn sbc(cpu: &mut CPU) {
        let opr1 = cpu.a as u16;
        let opr2 = match cpu.addr_mode {
                    AddrMode::Immediate => cpu.imm_val,
                    AddrMode::Accumulator => cpu.a,
                    AddrMode::EffAddr => cpu.mem.read(cpu.ea)
                    } as u16;
        let res = opr1 + (0xff - opr2) + cpu.get_carry() as u16;
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | OVER_FLAG | NEG_FLAG);
        cpu.a = res as u8;
        status |= (res > 0xff) as u8; /* carry flag */
        check_zero!(status, res);
        status |= (((opr1 ^ opr2) as u8 & 0x80) &
                    ((opr1 ^ res) as u8 & 0x80)) >> 1; /* over flag */
        check_neg!(status, res);
        cpu.status = status;
    }
    
    macro_rules! make_logic {
        ($f: ident, $op: tt) => (
        fn $f(cpu: &mut CPU) {
            let res = cpu.a $op match cpu.addr_mode {
                        AddrMode::Immediate => cpu.imm_val,
                        _ => cpu.mem.read(cpu.ea)
                    };
            let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
            cpu.a = res as u8;
            check_zero!(status, res);
            check_neg!(status, res);
            cpu.status = status;
        });
    }

    make_logic!(and, &);
    make_logic!(eor, ^);
    make_logic!(ora, |);

    fn asl(cpu: &mut CPU) {
        let res = match cpu.addr_mode {
                    AddrMode::Accumulator => {
                        let t = (cpu.a as u16) << 1;
                        cpu.a = t as u8;
                        t
                    },
                    _ => {
                        let t = (cpu.mem.read(cpu.ea) as u16) << 1;
                        cpu.mem.write(cpu.ea, t as u8);
                        t
                    }};
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
        status |= (res > 0xff) as u8; /* carry flag */
        check_zero!(status, res);
        check_neg!(status, res);
        cpu.status = status;
    }
    
    fn lsr(cpu: &mut CPU) {
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
        let res = match cpu.addr_mode {
                    AddrMode::Accumulator => {
                        let old = cpu.a;
                        let t = old >> 1;
                        cpu.a = t as u8;
                        status |= (old & 1) as u8; /* carry flag */
                        t
                    },
                    _ => {
                        let old = cpu.mem.read(cpu.ea);
                        let t = old >> 1;
                        cpu.mem.write(cpu.ea, t as u8);
                        status |= (old & 1) as u8; /* carry flag */
                        t
                    }};
        check_zero!(status, res);
        check_neg!(status, res);
        cpu.status = status;
    }

    fn rol(cpu: &mut CPU) {
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
        let res = match cpu.addr_mode {
                    AddrMode::Accumulator => {
                        let old = cpu.a;
                        let t = old.rotate_left(1);
                        cpu.a = t as u8;
                        status |= old >> 7 as u8; /* carry flag */
                        t
                    },
                    _ => {
                        let old = cpu.mem.read(cpu.ea);
                        let t = old.rotate_left(1);
                        cpu.mem.write(cpu.ea, t as u8);
                        status |= old >> 7 as u8; /* carry flag */
                        t
                    }};
        check_zero!(status, res);
        check_neg!(status, res);
        cpu.status = status;
    }

    fn ror(cpu: &mut CPU) {
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
        let res = match cpu.addr_mode {
                    AddrMode::Accumulator => {
                        let old = cpu.a;
                        let t = old.rotate_right(1);
                        cpu.a = t as u8;
                        status |= old & 1 as u8; /* carry flag */
                        t
                    },
                    _ => {
                        let old = cpu.mem.read(cpu.ea);
                        let t = old.rotate_right(1);
                        cpu.mem.write(cpu.ea, t as u8);
                        status |= old & 1 as u8; /* carry flag */
                        t
                    }};
        check_zero!(status, res);
        check_neg!(status, res);
        cpu.status = status;
    }

    fn bit(cpu: &mut CPU) {
        let m = cpu.mem.read(cpu.ea);
        let mut status = cpu.status & !(ZERO_FLAG | OVER_FLAG | NEG_FLAG);
        check_zero!(status, (m & cpu.a));
        status |= ((m >> 6) & 0x3) << 6; /* copy bit 6 & 7 */
        cpu.status = status;
    }

    macro_rules! make_branch_clear {
        ($f: ident, $e: ident) => (fn $f(cpu: &mut CPU) {
            match cpu.$e() {
                0 => {
                    cpu.pc = (cpu.pc as i32 + cpu.ea as i32) as u16;
                    cpu.cycle.wrapping_add(1);
                },
                _ => ()
            }});
    }

    macro_rules! make_branch_set {
        ($f: ident, $e: ident) => (fn $f(cpu: &mut CPU) {
            match cpu.$e() {
                0 => (),
                _ => {
                    cpu.pc = (cpu.pc as i32 + cpu.ea as i32) as u16;
                    cpu.cycle.wrapping_add(1);
                }
            }});
    }

    make_branch_clear!(bcc, get_carry);
    make_branch_set!(bcs, get_carry);
    make_branch_clear!(bne, get_zero);
    make_branch_set!(beq, get_zero);
    make_branch_clear!(bpl, get_neg);
    make_branch_set!(bmi, get_neg);
    make_branch_clear!(bvc, get_over);
    make_branch_set!(bvs, get_over);

    macro_rules! stack_addr {
        ($sp: ident, $disp: expr) => (($sp.wrapping_sub($disp) as u16) | 0x0100);
    }

    fn brk(cpu: &mut CPU) {
        let npc = cpu.pc.wrapping_add(2);
        let sp = cpu.sp;
        cpu.mem.write(stack_addr!(sp, 0), (npc >> 8) as u8); /* push high pc */
        cpu.mem.write(stack_addr!(sp, 1), npc as u8); /* push low pc */
        cpu.status |= BRK_FLAG;
        cpu.mem.write(stack_addr!(sp, 2), cpu.status); /* push status */
        cpu.sp = sp.wrapping_sub(3);
        cpu.pc = read16!(cpu.mem, 0xfffe as u16); /* load the interrupt vector */
    }

    fn clc(cpu: &mut CPU) { cpu.status &= !CARRY_FLAG; }
    fn cld(cpu: &mut CPU) { cpu.status &= !DEC_FLAG; }
    fn cli(cpu: &mut CPU) { cpu.status &= !INT_FLAG; }
    fn clv(cpu: &mut CPU) { cpu.status &= !OVER_FLAG; }

    fn sec(cpu: &mut CPU) { cpu.status |= CARRY_FLAG; }
    fn sed(cpu: &mut CPU) { cpu.status |= DEC_FLAG; }
    fn sei(cpu: &mut CPU) { cpu.status |= INT_FLAG; }


    macro_rules! make_cmp {
        ($f: ident, $r: ident) => (fn $f(cpu: &mut CPU) {
            let opr1 = cpu.$r as u16;
            let opr2 = match cpu.addr_mode {
                        AddrMode::Immediate => cpu.imm_val,
                        _ => cpu.mem.read(cpu.ea)
                        } as u16;
            let res = opr1 - opr2;
            let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
            status |= (res < 0x100) as u8; /* carry flag */
            check_zero!(status, res);
            check_neg!(status, res);
            cpu.status = status;
        });
    }

    make_cmp!(cmp, a);
    make_cmp!(cpx, x);
    make_cmp!(cpy, y);

    macro_rules! make_delta {
        ($f: ident, $d: expr) => (
            fn $f(cpu: &mut CPU) {
                let res = cpu.mem.read(cpu.ea) as u16 + $d;
                let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
                cpu.mem.write(cpu.ea, res as u8);
                check_zero!(status, res);
                check_neg!(status, res);
                cpu.status = status;
            });
        ($f: ident, $d: expr, $r: ident) => (
            fn $f(cpu: &mut CPU) {
                let res = cpu.$r as u16 + $d;
                let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
                cpu.$r = res as u8;
                check_zero!(status, res);
                check_neg!(status, res);
                cpu.status = status;
            });

    }

    make_delta!(dec, 0xff);
    make_delta!(dex, 0xff, x);
    make_delta!(dey, 0xff, y);
    make_delta!(inc, 0x01);
    make_delta!(inx, 0x01, x);
    make_delta!(iny, 0x01, y);

    fn jmp(cpu: &mut CPU) { cpu.pc = cpu.ea; }

    fn jsr(cpu: &mut CPU) {
        let sp = cpu.sp;
        let pc = cpu.pc.wrapping_sub(1);
        cpu.mem.write(stack_addr!(sp, 0), (pc >> 8) as u8);
        cpu.mem.write(stack_addr!(sp, 1), pc as u8);
        cpu.sp = sp.wrapping_sub(2);
        cpu.pc = cpu.ea;
    }

    macro_rules! make_ld {
        ($f: ident, $r: ident) => (fn $f(cpu: &mut CPU) {
            let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
            let res = cpu.mem.read(cpu.ea);
            cpu.a = res;
            check_zero!(status, res);
            check_neg!(status, res);
            cpu.status = status;
        });
    }

    make_ld!(lda, a);
    make_ld!(ldx, x);
    make_ld!(ldy, y);

    macro_rules! make_st {
        ($f: ident, $r: ident) => (fn $f(cpu: &mut CPU) {
            cpu.mem.write(cpu.ea, cpu.$r);
        });
    }

    make_st!(sta, a);
    make_st!(stx, x);
    make_st!(sty, y);

    macro_rules! make_trans {
        ($f: ident, $from: ident, $to: ident) => (fn $f(cpu: &mut CPU) {
            let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
            let res = cpu.$from;
            cpu.$to = res; 
            check_zero!(status, res);
            check_neg!(status, res);
            cpu.status = status;
        });
    }

    make_trans!(tax, a, x);
    make_trans!(tay, a, y);
    make_trans!(tsx, sp, x);
    make_trans!(txa, x, a);
    make_trans!(txs, x, sp);
    make_trans!(tya, y, a);

    fn pha(cpu: &mut CPU) {
        let sp = cpu.sp;
        cpu.mem.write(stack_addr!(sp, 0), cpu.a);
        cpu.sp = sp.wrapping_sub(1);
    }

    fn php(cpu: &mut CPU) {
        let sp = cpu.sp;
        cpu.mem.write(stack_addr!(sp, 0), cpu.status);
        cpu.sp = sp.wrapping_sub(1);
    }

    fn pla(cpu: &mut CPU) {
        let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
        let sp = cpu.sp.wrapping_add(1);
        let res = cpu.mem.read(stack_addr!(sp, 0));
        cpu.a = res;
        cpu.sp = sp;
        check_zero!(status, res);
        check_neg!(status, res);
        cpu.status = status;
    }

    fn plp(cpu: &mut CPU) {
        let sp = cpu.sp.wrapping_add(1);
        cpu.status = cpu.mem.read(stack_addr!(sp, 0));
        cpu.sp = sp;
    }

    fn rti(cpu: &mut CPU) {
        let sp = cpu.sp.wrapping_add(3);
        cpu.status = cpu.mem.read(stack_addr!(sp, 2));
        cpu.pc = make16!(cpu.mem.read(stack_addr!(sp, 0)),
                        cpu.mem.read(stack_addr!(sp, 1)));
        cpu.sp = sp;
    }

    fn rts(cpu: &mut CPU) {
        let sp = cpu.sp.wrapping_add(2);
        cpu.pc = make16!(cpu.mem.read(stack_addr!(sp, 0)),
                        cpu.mem.read(stack_addr!(sp, 1))).wrapping_add(1);
        cpu.sp = sp;
    }

    fn nop(_cpu: &mut CPU) {}
    fn nil(cpu: &mut CPU) {
        panic!("invalid instruction: 0x{:02x}", cpu.mem.read(cpu.pc));
    }
}

mod addr {
    use mos6502::{CPU, AddrMode};
    make_addrtable!(ADDR_MODES, fn (&mut CPU));

    fn acc(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::Accumulator;
    }

    fn imm(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::Immediate;
        cpu.imm_val = cpu.mem.read(cpu.pc.wrapping_add(1));
    }

    fn zpg(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        cpu.ea = cpu.mem.read(cpu.pc.wrapping_add(1)) as u16;
    }

    fn zpx(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        cpu.ea = (cpu.mem.read(cpu.pc.wrapping_add(1))
                        .wrapping_add(cpu.x)) as u16;
    }

    fn zpy(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        cpu.ea = (cpu.mem.read(cpu.pc.wrapping_add(1))
                        .wrapping_add(cpu.y)) as u16;
    }

    fn rel(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        cpu.ea = cpu.mem.read(cpu.pc.wrapping_add(1)) as u16;
    }

    fn abs(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        cpu.ea = read16!(cpu.mem, cpu.pc.wrapping_add(1));
    }

    fn abx(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        cpu.ea = read16!(cpu.mem, cpu.pc.wrapping_add(1))
                                .wrapping_add(cpu.x as u16);
    }

    fn aby(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        cpu.ea = read16!(cpu.mem, cpu.pc.wrapping_add(1))
                                .wrapping_add(cpu.y as u16);
    }

    fn ind(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        let addr = read16!(cpu.mem, cpu.pc.wrapping_add(1));
        cpu.ea = read16!(cpu.mem, addr);
    }

    fn xin(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        let addr = cpu.mem.read(cpu.mem.read(cpu.pc.wrapping_add(1))
                                    .wrapping_add(cpu.x) as u16) as u16;
        cpu.ea = read16!(cpu.mem, addr);
    }

    fn iny(cpu: &mut CPU) {
        cpu.addr_mode = AddrMode::EffAddr;
        let addr = cpu.mem.read(cpu.mem.read(cpu.pc.wrapping_add(1)) as u16) as u16;
        cpu.ea = read16!(cpu.mem, addr).wrapping_add(cpu.y as u16);
    }

    fn nil(_cpu: &mut CPU) {}
}

pub trait VMem {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

pub struct CPUMemory {
    internal: [u8; 2048]
}

impl CPUMemory {
    pub fn new() -> Self {
        CPUMemory{internal: [0; 2048]}
    }
}

impl VMem for CPUMemory {
    fn read(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            self.internal[(addr & 0x07ff) as usize]
        } else if addr < 0x4000 {
            match addr & 0x7 {
                _ => 0
            }
        } else {
            panic!("invalid memory read access at 0x{:04x}", addr)
        }
    }
    fn write(&mut self, addr: u16, data: u8) {
        if addr < 0x2000 {
            self.internal[(addr & 0x07ff) as usize] = data;
        } else if addr < 0x4000 {
            match addr & 0x7 {
                _ => ()
            }
        } else {
            panic!("invalid memory write access at 0x{:04x}", addr)
        }
    }
}

enum AddrMode {
    Immediate,
    Accumulator,
    EffAddr
}

pub struct CPU<'a> {
    /* registers */
    a: u8,
    x: u8,
    y: u8,
    status: u8,
    pc: u16,
    sp: u8,
    /* internal state */
    addr_mode: AddrMode,
    ea: u16,    /* effective address */
    imm_val: u8,
    cycle: u32,
    mem: &'a mut VMem
}

impl<'a> CPU<'a> {
    fn step(&mut self, opcode: u8) {
        ops::OPS[opcode as usize](self);
        addr::ADDR_MODES[opcode as usize](self);
    }
    #[inline(always)] pub fn get_carry(&self) -> u8 { (self.status >> 0) & 1 }
    #[inline(always)] pub fn get_zero(&self) -> u8 { (self.status >> 1) & 1 }
    #[inline(always)] pub fn get_over(&self) -> u8 { (self.status >> 6) & 1 }
    #[inline(always)] pub fn get_neg(&self) -> u8 { (self.status >> 7) & 1 }
    pub fn new(mem: &'a mut VMem) -> Self {
        CPU{a: 0, x: 0, y: 0, status: 0, pc: 0, sp: 0, ea: 0, cycle: 0,
            addr_mode: AddrMode::EffAddr,
            imm_val: 0,
            mem}
    }
}
