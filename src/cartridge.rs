use std::{cell::RefCell, rc::Weak};

use crate::{
    is_bit_set,
    mapper::{Mapper, Mapper0, Mirroring},
    Bus,
};

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
