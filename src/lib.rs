mod apu;
mod bus;
mod cartridge;
pub mod cpu;
pub mod mapper;
pub mod ppu;
mod replay;

#[cfg(feature = "wasm")]
use std::{cell::RefCell, rc::Rc};

pub use apu::Apu;
pub use bus::Bus;
pub use cartridge::Cartridge;
pub use cpu::Cpu;
pub use ppu::Ppu;
pub use replay::{InputCommand, Replay};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct Nes {
    bus: Rc<RefCell<Bus>>,
    cpu: Rc<RefCell<Cpu>>,
    ppu: Rc<RefCell<Ppu>>,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl Nes {
    pub fn new(rom: &[u8]) -> Result<Nes, String> {
        let cartridge = Rc::new(RefCell::new(Cartridge::new(rom)?));
        let cpu = Rc::new(RefCell::new(Cpu::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(cartridge.clone())));
        let apu = Rc::new(RefCell::new(Apu::new()));
        let bus = Bus::new(cpu.clone(), [0; 2048], ppu.clone(), apu, cartridge);
        cpu.borrow_mut().reset();

        Ok(Self { bus, cpu, ppu })
    }

    pub fn tick(&self) {
        while !self.ppu.borrow().is_frame_ready {
            self.clock();
        }
        self.ppu.borrow_mut().is_frame_ready = false;
    }

    pub fn image_buffer_raw(&self) -> *const u8 {
        self.ppu.borrow().buffer_raw()
    }

    pub fn set_controller_state(&self, controller_1: Controller, controller_2: Controller) {
        self.bus
            .borrow_mut()
            .set_controller_state(controller_1, controller_2);
    }

    fn clock(&self) {
        Bus::clock(self.bus.clone(), self.cpu.clone(), self.ppu.clone());
    }
}

#[inline]
pub const fn is_bit_set(byte: u8, index: u8) -> bool {
    (byte >> index & 1) != 0
}

#[inline]
pub const fn concat_bytes(low: u8, high: u8) -> u16 {
    (high as u16) << 8 | low as u16
}

#[inline]
pub const fn low_byte(word: u16) -> u8 {
    word as u8
}

#[inline]
pub const fn high_byte(word: u16) -> u8 {
    (word >> 8) as u8
}

#[bitfield_struct::bitfield(u8)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(PartialEq, Eq)]
pub struct Controller {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl Controller {
    // Necessary because the From<u8> trait implementation is inaccessible from Wasm.
    pub fn from_u8(value: u8) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let format_input = |input: bool, str: &'static str| if input { str } else { "." };

        let right = format_input(self.right(), "R");
        let left = format_input(self.left(), "L");
        let down = format_input(self.down(), "D");
        let up = format_input(self.up(), "U");
        let start = format_input(self.start(), "T");
        let select = format_input(self.select(), "S");
        let b = format_input(self.b(), "B");
        let a = format_input(self.a(), "A");

        write!(f, "{right}{left}{down}{up}{start}{select}{b}{a}")
    }
}
