#![allow(dead_code)]

pub trait Controller {
    fn read(&self) -> u8;
    fn write(&self, data: u8);
}

pub mod stdctl {
    use core::cell::Cell;
    use controller::Controller;
    #[derive(Copy, Clone)]
    pub enum Button {
        A = 0,
        B = 1,
        Select = 2,
        Start = 3,
        Up = 4,
        Down = 5,
        Left = 6,
        Right = 7,
        Null = 8,
    }
    pub struct Joystick {
        strobe: Cell<bool>,
        reg: Cell<u8>,
        back_reg: Cell<u8>
    }

    impl Joystick {
        pub fn new() -> Self {
            Joystick{reg: Cell::new(0), strobe: Cell::new(false), back_reg: Cell::new(0)}
        }

        pub fn set(&self, buttons: &[bool]) {
            let mut reg = 0;
            for (i, v) in buttons.iter().enumerate() {
                if *v {
                    reg |= 1 << i;
                }
            }
            self.reg.set(reg);
            self.back_reg.set(reg);
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
