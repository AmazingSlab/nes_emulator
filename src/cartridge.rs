#![allow(dead_code, unused_variables)]

use std::{cell::RefCell, rc::Weak};

use crate::{is_bit_set, Bus};

#[derive(Debug)]
pub struct Cartridge {
    mapper: Box<dyn Mapper>,
    bus: Weak<RefCell<Bus>>,
}

impl Cartridge {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        if &bytes[0..4] != b"NES\x1a" {
            return Err("not a nes file".into());
        }

        let _is_nes_20 = {
            let byte = bytes[7];
            !is_bit_set(byte, 2) && is_bit_set(byte, 3)
        };

        let prg_rom_blocks = bytes[4];
        let chr_rom_blocks = bytes[5];
        let mapper_id = bytes[6] >> 4 | (bytes[7] & 0xF0);

        let prg_rom_bytes = prg_rom_blocks as usize * 16 * 1024;
        let prg_rom = &bytes[16..prg_rom_bytes + 16];

        let mapper = match mapper_id {
            0 => Mapper0::new(prg_rom),
            id => return Err(format!("mapper {id} not implemented")),
        };

        Ok(Self {
            mapper: Box::new(mapper),
            bus: Weak::new(),
        })
    }

    pub fn connect_bus(&mut self, bus: Weak<RefCell<Bus>>) {
        self.bus = bus;
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.mapper.read(addr)
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.mapper.write(addr, data)
    }
}

pub trait Mapper: std::fmt::Debug {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, data: u8);
}

#[derive(Debug)]
pub struct Mapper0 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    variant: NromVariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NromVariant {
    Nrom128,
    Nrom256,
}

impl Mapper0 {
    pub fn new(prg_rom: &[u8]) -> Self {
        Self {
            prg_rom: prg_rom.into(),
            chr_rom: Vec::new(),
            variant: NromVariant::Nrom128,
        }
    }

    fn map_addr(&self, addr: u16) -> usize {
        let addr = addr as usize & 0x7FFF;
        match self.variant {
            NromVariant::Nrom128 => addr & 0x3FFF,
            NromVariant::Nrom256 => addr,
        }
    }
}

impl Mapper for Mapper0 {
    fn read(&self, addr: u16) -> u8 {
        let addr = self.map_addr(addr);
        self.prg_rom[addr]
    }

    fn write(&mut self, addr: u16, data: u8) {
        todo!()
    }
}