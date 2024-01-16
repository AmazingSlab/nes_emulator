mod bus;
mod cartridge;
pub mod cpu;
pub mod mapper;
pub mod ppu;
mod replay;

#[cfg(feature = "wasm")]
use std::{cell::RefCell, rc::Rc};

pub use bus::Bus;
pub use cartridge::Cartridge;
pub use cpu::Cpu;
pub use ppu::Ppu;
pub use replay::Replay;

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
        let bus = Bus::new(cpu.clone(), [0; 2048], ppu.clone(), cartridge);
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

#[cfg(not(feature = "wasm"))]
#[bitfield_struct::bitfield(u8)]
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
#[derive(Default, Clone, Copy)]
pub struct Controller(u8);

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl Controller {
    pub fn new(state: u8) -> Self {
        Self(state)
    }
}
