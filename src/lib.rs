#![allow(clippy::all)]

mod bus;
pub mod cpu;
mod memory;

pub use bus::Bus;
pub use cpu::Cpu;
pub use memory::Memory;

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
