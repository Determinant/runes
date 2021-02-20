#![allow(dead_code)]

mod disops {
    use runes::{ids2strs, make_optable};
    make_optable!(OPS, &str);
    ids2strs!(
        adc, and, asl, bcc, bcs, beq, bit, bmi, bne, bpl, brk, bvc, bvs, clc,
        cld, cli, clv, cmp, cpx, cpy, dec, dex, dey, eor, inc, inx, iny, jmp,
        jsr, lda, ldx, ldy, lsr, nop, ora, pha, php, pla, plp, rol, ror, rti,
        rts, sbc, sec, sed, sei, sta, stx, sty, tax, tay, tsx, txa, txs, tya,
        nil
    );
}

mod disaddr {
    use runes::make_addrtable;
    pub type T<'a, 'b> = &'a mut dyn Iterator<Item = &'b u8>;
    make_addrtable!(ADDR_MODES, fn(T) -> String);
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

pub struct Disassembler<'a, T>
where
    T: Iterator<Item = &'a u8>,
{
    raw_code: T,
}

impl<'a, T> Disassembler<'a, T>
where
    T: Iterator<Item = &'a u8>,
{
    pub fn new(raw_code: T) -> Self {
        Disassembler { raw_code }
    }
    fn parse(opcode: u8, code: &mut T) -> String {
        format!(
            "{} {}",
            disops::OPS[opcode as usize],
            disaddr::ADDR_MODES[opcode as usize](code)
        )
    }
}

impl<'a, T> Iterator for Disassembler<'a, T>
where
    T: Iterator<Item = &'a u8>,
{
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        match self.raw_code.next() {
            Some(opcode) => {
                Some(Disassembler::parse(*opcode, &mut self.raw_code))
            }
            None => None,
        }
    }
}

pub fn parse(opcode: u8, code: &[u8]) -> String {
    Disassembler::parse(opcode, &mut code.iter())
}
