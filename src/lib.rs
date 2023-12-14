mod bus;
mod cartridge;
pub mod cpu;
pub mod ppu;

pub use bus::Bus;
pub use cartridge::Cartridge;
pub use cpu::Cpu;
pub use ppu::Ppu;

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
