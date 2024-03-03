mod apu;
mod bus;
mod cartridge;
pub mod cpu;
mod game_genie;
pub mod mapper;
pub mod ppu;
mod replay;
pub mod savestate;

#[cfg(feature = "wasm")]
use std::{cell::RefCell, rc::Rc};

pub use apu::Apu;
pub use bus::Bus;
pub use cartridge::Cartridge;
pub use cpu::Cpu;
pub use game_genie::{GameGenie, GameGenieCode};
pub use ppu::Ppu;
pub use replay::{InputCommand, Replay};
pub use savestate::Savestate;

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
    apu: Rc<RefCell<Apu>>,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl Nes {
    pub fn new(rom: &[u8]) -> Result<Nes, String> {
        let cartridge = Rc::new(RefCell::new(Cartridge::new(rom)?));
        let cpu = Rc::new(RefCell::new(Cpu::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(cartridge.clone())));
        let apu = Rc::new(RefCell::new(Apu::new()));
        let bus = Bus::new(cpu.clone(), [0; 2048], ppu.clone(), apu.clone(), cartridge);
        cpu.borrow_mut().reset();

        Ok(Self { bus, cpu, ppu, apu })
    }

    pub fn tick(&self) {
        while !self.ppu.borrow().is_frame_ready {
            self.clock();
        }
        self.ppu.borrow_mut().is_frame_ready = false;
    }

    pub fn apply_state(&self, state: &[u8]) -> Result<(), String> {
        let decompressed = Savestate::decompress(state)?;
        let savestate = Savestate::new(&decompressed)?;

        self.bus.borrow_mut().apply_state(savestate);

        Ok(())
    }

    pub fn save_state(&self) -> Vec<u8> {
        self.bus.borrow().save_state()
    }

    pub fn image_buffer_raw(&self) -> *const u8 {
        self.ppu.borrow().buffer_raw()
    }

    pub fn drain_audio_buffer(&mut self) {
        self.apu.borrow_mut().drain_audio_buffer();
    }

    pub fn audio_buffer_raw(&mut self) -> *const f32 {
        self.apu.borrow_mut().audio_buffer().as_ptr()
    }

    pub fn audio_buffer_length(&self) -> usize {
        self.apu.borrow().audio_buffer_length()
    }

    pub fn set_controller_state(&self, controller_1: Controller, controller_2: Controller) {
        self.bus
            .borrow_mut()
            .set_controller_state(controller_1, controller_2);
    }

    fn clock(&self) {
        Bus::clock(
            self.bus.clone(),
            self.cpu.clone(),
            self.ppu.clone(),
            self.apu.clone(),
        );
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

/// Creates a new array directly on the heap without going through the stack.
///
/// This is a workaround to avoid stack overflows in debug builds, as without optimizations,
/// `Box::new([T; N])` allocates the array on the stack before moving to the heap.
pub fn new_boxed_array<T: Default + Clone, const N: usize>() -> Box<[T; N]> {
    // SAFETY: A Box<[T]> obtained from a Vec<T> with N elements is guaranteed to be safe to cast to
    // a Box<[T; N]>.
    unsafe { Box::from_raw(Box::into_raw(vec![T::default(); N].into_boxed_slice()) as *mut [T; N]) }
}
