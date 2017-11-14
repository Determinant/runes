#![allow(dead_code)]
use memory::VMem;
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
        nil, xin, nil, xin, zpg, zpg, zpg, zpg, nil, imm, acc, imm, abs, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpx, zpx, nil, aby, nil, aby, abx, abx, abx, abx,
        abs, xin, nil, xin, zpg, zpg, zpg, zpg, nil, imm, acc, imm, abs, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpx, zpx, nil, aby, nil, aby, abx, abx, abx, abx,
        nil, xin, nil, xin, zpg, zpg, zpg, zpg, nil, imm, acc, imm, abs, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpx, zpx, nil, aby, nil, aby, abx, abx, abx, abx,
        nil, xin, nil, xin, zpg, zpg, zpg, zpg, nil, imm, acc, imm, ind, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpx, zpx, nil, aby, nil, aby, abx, abx, abx, abx,
        imm, xin, imm, xin, zpg, zpg, zpg, zpg, nil, imm, nil, imm, abs, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpy, zpy, nil, aby, nil, aby, abx, abx, aby, aby,
        imm, xin, imm, xin, zpg, zpg, zpg, zpg, nil, imm, nil, imm, abs, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpy, zpy, nil, aby, nil, aby, abx, abx, aby, aby,
        imm, xin, imm, xin, zpg, zpg, zpg, zpg, nil, imm, nil, imm, abs, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpx, zpx, nil, aby, nil, aby, abx, abx, abx, abx,
        imm, xin, imm, xin, zpg, zpg, zpg, zpg, nil, imm, nil, imm, abs, abs, abs, abs,
        rel, iny, nil, iny, zpx, zpx, zpx, zpx, nil, aby, nil, aby, abx, abx, abx, abx,
    ];);
}

const INST_LENGTH: [u8; 0x100] =  [
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    3, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    1, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 0, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 0, 3, 0, 0,
    2, 2, 2, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 2, 1, 0, 3, 3, 3, 0,
    2, 2, 0, 0, 2, 2, 2, 0, 1, 3, 1, 0, 3, 3, 3, 0,
];

const INST_CYCLE: [u8; 0x100] = [
    7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6,
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6,
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6,
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6,
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
];

const NMI_VECTOR: u16 = 0xfffa;
const RESET_VECTOR: u16 = 0xfffc;
const IRQ_VECTOR: u16 = 0xfffe;
const BRK_VECTOR: u16 = 0xfffe;

const CARRY_FLAG: u8 = 1 << 0;
const ZERO_FLAG: u8 = 1 << 1;
const INT_FLAG: u8 = 1 << 2;
const DEC_FLAG: u8 = 1 << 3;
const BRK_FLAG: u8 = 1 << 4;
const OVER_FLAG: u8 = 1 << 6;
const NEG_FLAG: u8 = 1 << 7;

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
            let b = *code.next().unwrap() as i8 as i16;
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

macro_rules! stack_addr {
    ($sp: ident, $disp: expr) => (($sp.wrapping_sub($disp) as u16) | 0x0100);
}

macro_rules! make16 {
    ($high: expr, $low: expr) => ((($high as u16) << 8) | ($low as u16));
}

macro_rules! read16 {
    ($mem: expr, $laddr: expr) => (make16!($mem.read($laddr.wrapping_add(1)),
                                                $mem.read($laddr)));
}

mod ops {
    use mos6502::*;
    make_optable!(OPS, fn (&mut CPU));

    macro_rules! check_zero {
        ($st: ident, $r: expr) => ($st |= (($r as u8 == 0) as u8) << 1);
    }
    macro_rules! check_neg {
        ($st: ident, $r: expr) => ($st |= ($r as u8 & NEG_FLAG) as u8);
    }

    /* arithmetic */

    fn adc(cpu: &mut CPU) {
        let opr1 = cpu.a as u16;
        let opr2 = cpu.mem.read(cpu.ea) as u16;
        let res = opr1 + opr2 + (cpu.get_carry() as u16);
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
        let opr2 = cpu.mem.read(cpu.ea) as u16;
        let res = opr1 + (0xff - opr2) + (cpu.get_carry() as u16);
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | OVER_FLAG | NEG_FLAG);
        cpu.a = res as u8;
        status |= (res > 0xff) as u8; /* carry flag */
        check_zero!(status, res);
        status |= (((opr1 ^ opr2) as u8 & 0x80) &
                    ((opr1 ^ res) as u8 & 0x80)) >> 1; /* over flag */
        check_neg!(status, res);
        cpu.status = status;
    }
    
    macro_rules! make_cmp {
        ($f: ident, $r: ident) => (fn $f(cpu: &mut CPU) {
            let opr1 = cpu.$r as u16;
            let opr2 = cpu.mem.read(cpu.ea) as u16;
            let res = opr1.wrapping_sub(opr2);
            let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
            status |= (res < 0x100) as u8; /* if opr1 >= opr2 */
            check_zero!(status, res);
            check_neg!(status, res);
            cpu.status = status;
        });
    }

    make_cmp!(cmp, a);
    make_cmp!(cpx, x);
    make_cmp!(cpy, y);

    /* increments & decrements */
    macro_rules! make_delta {
        ($f: ident, $d: expr) => (
            fn $f(cpu: &mut CPU) {
                let res = cpu.mem.read(cpu.ea).wrapping_add($d);
                let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
                cpu.mem.write(cpu.ea, res);
                check_zero!(status, res);
                check_neg!(status, res);
                cpu.status = status;
            });
        ($f: ident, $d: expr, $r: ident) => (
            fn $f(cpu: &mut CPU) {
                let res = cpu.$r.wrapping_add($d);
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

    /* logical */
    macro_rules! make_logic {
        ($f: ident, $op: tt) => (
        fn $f(cpu: &mut CPU) {
            let res = cpu.a $op cpu.mem.read(cpu.ea);
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

    fn bit(cpu: &mut CPU) {
        let m = cpu.mem.read(cpu.ea);
        let mut status = cpu.status & !(ZERO_FLAG | OVER_FLAG | NEG_FLAG);
        check_zero!(status, (m & cpu.a));
        status |= ((m >> 6) & 0x3) << 6; /* copy bit 6 & 7 */
        cpu.status = status;
    }

    /* shifts */
    fn asl(cpu: &mut CPU) {
        let res = match cpu.acc {
                    true => {
                        let t = (cpu.a as u16) << 1;
                        cpu.a = t as u8;
                        t
                    },
                    false => {
                        let t = (cpu.mem.read(cpu.ea) as u16) << 1;
                        cpu.mem.write(cpu.ea, t as u8);
                        t
                    }};
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
        status |= (res >> 8) as u8; /* carry flag */
        check_zero!(status, res);
        check_neg!(status, res);
        cpu.status = status;
    }
    
    fn lsr(cpu: &mut CPU) {
        let mut status = cpu.status & !(CARRY_FLAG | ZERO_FLAG | NEG_FLAG);
        let res = match cpu.acc {
                    true => {
                        let old = cpu.a;
                        let t = old >> 1;
                        cpu.a = t as u8;
                        status |= (old & 1) as u8; /* carry flag */
                        t
                    },
                    false => {
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
        let res = match cpu.acc {
                    true => {
                        let old = cpu.a;
                        let t = old.rotate_left(1);
                        cpu.a = t as u8;
                        status |= old >> 7 as u8; /* carry flag */
                        t
                    },
                    false => {
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
        let res = match cpu.acc {
                    true => {
                        let old = cpu.a;
                        let t = old.rotate_right(1);
                        cpu.a = t as u8;
                        status |= old & 1 as u8; /* carry flag */
                        t
                    },
                    false => {
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

    /* branches */
    macro_rules! make_branch_clear {
        ($f: ident, $e: ident) => (fn $f(cpu: &mut CPU) {
            match cpu.$e() {
                0 => {
                    cpu.pc = cpu.ea;
                    cpu.cycle += 1;
                },
                _ => ()
            }});
    }

    macro_rules! make_branch_set {
        ($f: ident, $e: ident) => (fn $f(cpu: &mut CPU) {
            match cpu.$e() {
                0 => (),
                _ => {
                    cpu.pc = cpu.ea;
                    cpu.cycle += 1;
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

    fn brk(cpu: &mut CPU) {
        let pc = cpu.pc;
        let sp = cpu.sp;
        cpu.mem.write(stack_addr!(sp, 0), (pc >> 8) as u8); /* push high pc */
        cpu.mem.write(stack_addr!(sp, 1), pc as u8); /* push low pc */
        cpu.status |= BRK_FLAG;
        cpu.mem.write(stack_addr!(sp, 2), cpu.status); /* push status */
        cpu.status |= INT_FLAG;
        cpu.sp = sp.wrapping_sub(3);
        cpu.pc = read16!(cpu.mem, BRK_VECTOR); /* load the interrupt vector */
    }

    /* status flag changes */
    fn clc(cpu: &mut CPU) { cpu.status &= !CARRY_FLAG; }
    fn cld(cpu: &mut CPU) { cpu.status &= !DEC_FLAG; }
    fn cli(cpu: &mut CPU) { cpu.status &= !INT_FLAG; }
    fn clv(cpu: &mut CPU) { cpu.status &= !OVER_FLAG; }

    fn sec(cpu: &mut CPU) { cpu.status |= CARRY_FLAG; }
    fn sed(cpu: &mut CPU) { cpu.status |= DEC_FLAG; }
    fn sei(cpu: &mut CPU) { cpu.status |= INT_FLAG; }

    /* jumps & calls */
    fn jmp(cpu: &mut CPU) { cpu.pc = cpu.ea; }

    fn jsr(cpu: &mut CPU) {
        let sp = cpu.sp;
        let pc = cpu.pc.wrapping_sub(1);
        cpu.mem.write(stack_addr!(sp, 0), (pc >> 8) as u8);
        cpu.mem.write(stack_addr!(sp, 1), pc as u8);
        cpu.sp = sp.wrapping_sub(2);
        cpu.pc = cpu.ea;
    }

    fn rts(cpu: &mut CPU) {
        let sp = cpu.sp.wrapping_add(2);
        cpu.pc = make16!(cpu.mem.read(stack_addr!(sp, 0)),
                        cpu.mem.read(stack_addr!(sp, 1))).wrapping_add(1);
        cpu.sp = sp;
    }

    /* system functions */
    fn rti(cpu: &mut CPU) {
        let sp = cpu.sp.wrapping_add(3);
        cpu.status = cpu.mem.read(stack_addr!(sp, 2));
        cpu.pc = make16!(cpu.mem.read(stack_addr!(sp, 0)),
                        cpu.mem.read(stack_addr!(sp, 1)));
        cpu.sp = sp;
    }

    fn nop(_cpu: &mut CPU) {}
    
    /* load/store operations */
    macro_rules! make_ld {
        ($f: ident, $r: ident) => (fn $f(cpu: &mut CPU) {
            let mut status = cpu.status & !(ZERO_FLAG | NEG_FLAG);
            let res = cpu.mem.read(cpu.ea);
            cpu.$r = res;
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

    /* register transfers */
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
    make_trans!(txa, x, a);
    make_trans!(tya, y, a);

    /* stack operations */
    make_trans!(tsx, sp, x);
    fn txs(cpu: &mut CPU) { cpu.sp = cpu.x; }

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

    fn nil(cpu: &mut CPU) {
        panic!("invalid instruction: 0x{:02x}", cpu.mem.read(cpu.pc));
    }
}

mod addr {
    use mos6502::{CPU};
    make_addrtable!(ADDR_MODES, fn (&mut CPU));

    fn acc(cpu: &mut CPU) {
        cpu.acc = true;
    }

    fn imm(cpu: &mut CPU) {
        cpu.ea = cpu.opr;
    }

    fn zpg(cpu: &mut CPU) {
        cpu.ea = cpu.mem.read(cpu.opr) as u16;
    }

    fn zpx(cpu: &mut CPU) {
        cpu.ea = (cpu.mem.read(cpu.opr)
                         .wrapping_add(cpu.x)) as u16;
    }

    fn zpy(cpu: &mut CPU) {
        cpu.ea = (cpu.mem.read(cpu.opr)
                         .wrapping_add(cpu.y)) as u16;
    }

    fn rel(cpu: &mut CPU) {
        let base = cpu.pc;
        let offset = cpu.mem.read(cpu.opr) as i8 as i16;
        let sum = ((base & 0xff) as i16 + offset) as u16;
        cpu.ea = (base & 0xff00).wrapping_add(sum);
        cpu.cycle += (sum >> 8) as u32;
    }

    fn abs(cpu: &mut CPU) {
        cpu.ea = read16!(cpu.mem, cpu.opr);
    }

    fn abx(cpu: &mut CPU) {
        let base = read16!(cpu.mem, cpu.opr);
        let sum = (base & 0xff) + (cpu.x as u16);
        cpu.ea = (base & 0xff00).wrapping_add(sum);
        cpu.cycle += (sum >> 8) as u32; /* boundary cross if carry */
    }

    fn aby(cpu: &mut CPU) {
        let base = read16!(cpu.mem, cpu.opr);
        let sum = (base & 0xff) + (cpu.y as u16);
        cpu.ea = (base & 0xff00).wrapping_add(sum);
        cpu.cycle += (sum >> 8) as u32; /* boundary cross if carry */
    }

    fn ind(cpu: &mut CPU) {
        let addr = read16!(cpu.mem, cpu.opr);
        cpu.ea = read16!(cpu.mem, addr);
    }

    fn xin(cpu: &mut CPU) {
        cpu.ea = read16!(cpu.mem,
                         cpu.mem.read(cpu.opr)
                                .wrapping_add(cpu.x) as u16) as u16;
    }

    fn iny(cpu: &mut CPU) {
        let base = read16!(cpu.mem, cpu.mem.read(cpu.opr) as u16);
        let sum = (base & 0xff) + (cpu.y as u16);
        cpu.ea = (base & 0xff00).wrapping_add(sum);
        cpu.cycle += (sum >> 8) as u32; /* boundary cross if carry */
    }

    fn nil(_cpu: &mut CPU) {}
}

enum IntType {
    NMI,
    IRQ
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
    acc: bool,
    opr: u16,
    ea: u16,    /* effective address */
    imm_val: u8,
    pub cycle: u32,
    int: Option<IntType>,
    pub mem: &'a VMem
}

macro_rules! make_int {
    ($f:ident, $v: expr) => (
    fn $f(&mut self) {
        let pc = self.pc;
        let sp = self.sp;
        self.mem.write(stack_addr!(sp, 0), (pc >> 8) as u8);
        self.mem.write(stack_addr!(sp, 1), pc as u8);
        self.mem.write(stack_addr!(sp, 2), self.status);
        self.sp = sp.wrapping_sub(3);
        self.pc = read16!(self.mem, $v as u16);
        self.status |= INT_FLAG;
        self.cycle += 7;
    });
}

impl<'a> CPU<'a> {
    #[inline(always)] pub fn get_carry(&self) -> u8 { (self.status >> 0) & 1 }
    #[inline(always)] pub fn get_zero(&self) -> u8 { (self.status >> 1) & 1 }
    #[inline(always)] pub fn get_int(&self) -> u8 { (self.status >> 2) & 1 }
    #[inline(always)] pub fn get_over(&self) -> u8 { (self.status >> 6) & 1 }
    #[inline(always)] pub fn get_neg(&self) -> u8 { (self.status >> 7) & 1 }

    pub fn new(mem: &'a VMem) -> Self {
        let pc = read16!(mem, RESET_VECTOR as u16);
        /* nes power up state */
        let a = 0;
        let x = 0;
        let y = 0;
        let sp = 0xfd;
        let status = 0x34;
        let cycle = 0;

        CPU{a, x, y,
            pc, sp, status, cycle,
            opr: 0, ea: 0, imm_val: 0,
            int: None,
            acc: false,
            mem}
    }

    make_int!(nmi, NMI_VECTOR);
    make_int!(irq, IRQ_VECTOR);

    pub fn step(&mut self) {
        match self.int {
            Some(IntType::NMI) => self.nmi(),
            Some(IntType::IRQ) => self.irq(),
            _ => ()
        }
        self.int = None;
        let pc = self.pc;
        let opcode = self.mem.read(pc) as usize;
        let len = INST_LENGTH[opcode];
        let mut code = vec![0; len as usize];
        for i in 0..len as u16 {
            code[i as usize] = self.mem.read(pc + i);
        }
        println!("0x{:04x} {} a:{} x:{} y:{}",
                 pc, disasm::parse(opcode as u8, &code[1..]), self.a, self.x, self.y);
        /* update opr pointing to operands of current inst */
        self.opr = pc.wrapping_add(1);
        /* update program counter pointing to next inst */
        self.pc = pc.wrapping_add(INST_LENGTH[opcode] as u16);
        /* get effective address based on addressing mode */
        self.acc = false;
        addr::ADDR_MODES[opcode](self);
        /* execute the inst */
        ops::OPS[opcode](self);
        self.cycle += INST_CYCLE[opcode] as u32;
    }

    pub fn get_pc(&self) -> u16 { self.pc }

    pub fn reset(&mut self) {
        self.pc = read16!(self.mem, RESET_VECTOR as u16);
        self.sp = self.sp.wrapping_sub(3);
        self.status |= INT_FLAG;
        self.cycle = 0;
        self.int = None;
    }

    pub fn trigger_nmi(&mut self) {
        self.int = Some(IntType::NMI);
    }

    pub fn trigger_irq(&mut self) {
        if self.get_int() == 0 {
            self.int = Some(match self.int {
                Some(IntType::NMI) => IntType::NMI,
                _ => IntType::IRQ
            });
        }
    }
}
