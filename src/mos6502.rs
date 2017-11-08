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
        fn acc(code: T) -> String {
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
            let b = *code.next().unwrap();
            if b >> 7 == 0 {
                format!("+${:02x}, x", b & 0x7f)
            } else {
                format!("-${:02x}, x", b & 0x7f)
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
        fn nil(code: T) -> String {
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

    mod ops {
        use mos6502::CPU;
        make_optable!(OPS, fn (&mut CPU));
    
        fn adc(cpu: &mut CPU) {}
        fn and(cpu: &mut CPU) {}
        fn asl(cpu: &mut CPU) {}
        fn bcc(cpu: &mut CPU) {}
        fn bcs(cpu: &mut CPU) {}
        fn beq(cpu: &mut CPU) {}
        fn bit(cpu: &mut CPU) {}
        fn bmi(cpu: &mut CPU) {}
        fn bne(cpu: &mut CPU) {}
        fn bpl(cpu: &mut CPU) {}
        fn brk(cpu: &mut CPU) {}
        fn bvc(cpu: &mut CPU) {}
        fn bvs(cpu: &mut CPU) {}
        fn clc(cpu: &mut CPU) {}
        fn cld(cpu: &mut CPU) {}
        fn cli(cpu: &mut CPU) {}
        fn clv(cpu: &mut CPU) {}
        fn cmp(cpu: &mut CPU) {}
        fn cpx(cpu: &mut CPU) {}
        fn cpy(cpu: &mut CPU) {}
        fn dec(cpu: &mut CPU) {}
        fn dex(cpu: &mut CPU) {}
        fn dey(cpu: &mut CPU) {}
        fn eor(cpu: &mut CPU) {}
        fn inc(cpu: &mut CPU) {}
        fn inx(cpu: &mut CPU) {}
        fn iny(cpu: &mut CPU) {}
        fn jmp(cpu: &mut CPU) {}
        fn jsr(cpu: &mut CPU) {}
        fn lda(cpu: &mut CPU) {}
        fn ldx(cpu: &mut CPU) {}
        fn ldy(cpu: &mut CPU) {}
        fn lsr(cpu: &mut CPU) {}
        fn nop(cpu: &mut CPU) {}
        fn ora(cpu: &mut CPU) {}
        fn pha(cpu: &mut CPU) {}
        fn php(cpu: &mut CPU) {}
        fn pla(cpu: &mut CPU) {}
        fn plp(cpu: &mut CPU) {}
        fn rol(cpu: &mut CPU) {}
        fn ror(cpu: &mut CPU) {}
        fn rti(cpu: &mut CPU) {}
        fn rts(cpu: &mut CPU) {}
        fn sbc(cpu: &mut CPU) {}
        fn sec(cpu: &mut CPU) {}
        fn sed(cpu: &mut CPU) {}
        fn sei(cpu: &mut CPU) {}
        fn sta(cpu: &mut CPU) {}
        fn stx(cpu: &mut CPU) {}
        fn sty(cpu: &mut CPU) {}
        fn tax(cpu: &mut CPU) {}
        fn tay(cpu: &mut CPU) {}
        fn tsx(cpu: &mut CPU) {}
        fn txa(cpu: &mut CPU) {}
        fn txs(cpu: &mut CPU) {}
        fn tya(cpu: &mut CPU) {}
        fn nil(cpu: &mut CPU) {}
    }
    
    mod addr {
        use mos6502::CPU;
        make_addrtable!(ADDR_MODES, fn (&mut CPU));
    
        fn acc(cpu: &mut CPU) {}
        fn imm(cpu: &mut CPU) {}
        fn zpg(cpu: &mut CPU) {}
        fn zpx(cpu: &mut CPU) {}
        fn zpy(cpu: &mut CPU) {}
        fn rel(cpu: &mut CPU) {}
        fn abs(cpu: &mut CPU) {}
        fn abx(cpu: &mut CPU) {}
        fn aby(cpu: &mut CPU) {}
        fn ind(cpu: &mut CPU) {}
        fn xin(cpu: &mut CPU) {}
        fn iny(cpu: &mut CPU) {}
        fn nil(cpu: &mut CPU) {}
    }
    
    pub trait VMem {
        fn read(addr: u16) -> u8;
        fn write(addr: u16, data: u8);
    }
    
    pub struct CPU {
        /* registers */
        a: u8,
        x: u8,
        y: u8,
        status: u8,
        pc: u8,
        sp: u8,
        /* internal state */
        ea: u16,    /* effective address */
    }
    
    impl CPU {
        fn step(&mut self, opcode: u8) {
            ops::OPS[opcode as usize](self);
            addr::ADDR_MODES[opcode as usize](self);
        }
        fn new() -> Self {
            CPU{a: 0, x: 0, y: 0, status: 0, pc: 0, sp: 0, ea: 0}
        }
    }

