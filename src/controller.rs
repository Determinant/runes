#![allow(dead_code)]

pub trait Controller {
    fn read(&self) -> u8;
    fn write(&self, data: u8);
}

pub mod stdctl {
    use core::cell::Cell;
    use controller::Controller;
    pub const A: u8 = 1 << 0;
    pub const B: u8 = 1 << 1;
    pub const SELECT: u8 = 1 << 2;
    pub const START: u8 = 1 << 3;
    pub const UP: u8 = 1 << 4;
    pub const DOWN: u8 = 1 << 5;
    pub const LEFT: u8 = 1 << 6;
    pub const RIGHT: u8 = 1 << 7;
    pub const NULL: u8 = 0;
    
    pub struct Joystick {
        strobe: Cell<bool>,
        reg: Cell<u8>,
        back_reg: Cell<u8>
    }

    impl Joystick {
        pub fn new() -> Self {
            Joystick{reg: Cell::new(0), strobe: Cell::new(false), back_reg: Cell::new(0)}
        }

        pub fn set(&self, buttons: u8) {
            self.reg.set(buttons);
            self.back_reg.set(buttons);
        }
    }

    impl Controller for Joystick {
        fn read(&self) -> u8 {
            let res = self.reg.get() & 1;
            if !self.strobe.get() {
                self.reg.set(self.reg.get() >> 1);
            }
            res
        }
        
        fn write(&self, data: u8) {
            self.strobe.set(data & 1 == 1);
            self.reg.set(self.back_reg.get());
        }
    }
}
