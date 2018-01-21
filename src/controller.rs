#![allow(dead_code)]
use utils::{Read, Write};

pub trait Controller {
    fn read(&self) -> u8;
    fn write(&self, data: u8);
    fn load(&mut self, reader: &mut Read) -> bool;
    fn save(&self, writer: &mut Write) -> bool;
}

pub trait InputPoller {
    fn poll(&self) -> u8;
}

pub mod stdctl {
    use utils::{Read, Write, load_prefix, save_prefix};
    use core::cell::Cell;
    use controller::{Controller, InputPoller};
    pub const A: u8 = 1 << 0;
    pub const B: u8 = 1 << 1;
    pub const SELECT: u8 = 1 << 2;
    pub const START: u8 = 1 << 3;
    pub const UP: u8 = 1 << 4;
    pub const DOWN: u8 = 1 << 5;
    pub const LEFT: u8 = 1 << 6;
    pub const RIGHT: u8 = 1 << 7;
    pub const NULL: u8 = 0;
    
    #[repr(C)]
    pub struct Joystick<'a> {
        strobe: Cell<bool>,
        reg: Cell<u8>,
        poller: &'a InputPoller,
    }

    impl<'a> Joystick<'a> {
        pub fn new(poller: &'a InputPoller) -> Self {
            Joystick{
                reg: Cell::new(0),
                strobe: Cell::new(false),
                poller
            }
        }
    }

    impl<'a> Controller for Joystick<'a> {
        fn read(&self) -> u8 {
            if self.strobe.get() {
                self.reg.set(self.poller.poll());
                self.reg.get() & 1
            } else {
                let old = self.reg.get();
                self.reg.set(old >> 1);
                old & 1
            }
        }
        
        fn write(&self, data: u8) {
            self.strobe.set(data & 1 == 1);
            if self.strobe.get() {
                self.reg.set(self.poller.poll())
            }
        }

        fn load(&mut self, reader: &mut Read) -> bool {
            load_prefix(self, 0, reader)
        }

        fn save(&self, writer: &mut Write) -> bool {
            save_prefix(self, 0, writer)
        }
    }
}
