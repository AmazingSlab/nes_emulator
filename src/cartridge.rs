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
        let mirror_flag = bytes[6] & 0x01;

        let prg_rom_bytes = prg_rom_blocks as usize * 16 * 1024;
        let prg_rom = &bytes[16..prg_rom_bytes + 16];
        let chr_rom_bytes = chr_rom_blocks as usize * 8 * 1024;
        let chr_rom = &bytes[prg_rom_bytes + 16..prg_rom_bytes + 16 + chr_rom_bytes];

        let mapper = match mapper_id {
            0 => Mapper0::new(prg_rom, chr_rom, prg_rom_blocks, mirror_flag)?,
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

    pub fn cpu_read(&self, addr: u16) -> u8 {
        self.mapper.cpu_read(addr)
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        self.mapper.cpu_write(addr, data)
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        self.mapper.ppu_read(addr)
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        self.mapper.ppu_write(addr, data)
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring()
    }
}

pub trait Mapper: std::fmt::Debug {
    fn cpu_read(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, data: u8);
    fn ppu_read(&self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, data: u8);
    fn mirroring(&self) -> Mirroring;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub struct Mapper0 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    variant: NromVariant,
    mirror_flag: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NromVariant {
    Nrom128,
    Nrom256,
}

impl Mapper0 {
    pub fn new(
        prg_rom: &[u8],
        chr_rom: &[u8],
        prg_rom_blocks: u8,
        mirror_flag: u8,
    ) -> Result<Self, String> {
        let variant = match prg_rom_blocks {
            1 => NromVariant::Nrom128,
            2 => NromVariant::Nrom256,
            blocks => return Err(format!("{blocks} is not a valid block size for mapper 0")),
        };

        Ok(Self {
            prg_rom: prg_rom.into(),
            chr_rom: chr_rom.into(),
            variant,
            mirror_flag,
        })
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
    fn cpu_read(&self, addr: u16) -> u8 {
        let addr = self.map_addr(addr);
        self.prg_rom[addr]
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        todo!()
    }

    fn ppu_read(&self, addr: u16) -> u8 {
        if !self.chr_rom.is_empty() {
            let addr = addr as usize & 0x1FFF;
            self.chr_rom[addr]
        } else {
            0
        }
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        todo!()
    }

    fn mirroring(&self) -> Mirroring {
        if self.mirror_flag == 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        }
    }
}
