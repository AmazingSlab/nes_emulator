use std::{cell::RefCell, rc::Weak};

use crate::{
    is_bit_set,
    mapper::{Mapper, Mapper0, Mapper1, Mapper4, Mirroring},
    Bus,
};

pub struct Cartridge {
    mapper: Box<dyn Mapper>,
    bus: Weak<RefCell<Bus>>,
}

impl Cartridge {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        if &bytes[0..4] != b"NES\x1a" {
            return Err("not a nes file".into());
        }

        let rom_info = RomInfo::new(&bytes[0..16].try_into().unwrap());
        println!("rom info:\n{rom_info}");

        let prg_rom_blocks = rom_info.prg_rom_blocks;
        let chr_rom_blocks = rom_info.chr_rom_blocks;
        let mapper_id = rom_info.mapper_id;
        let mirror_flag = rom_info.mirror_flag;

        let prg_rom_bytes = prg_rom_blocks as usize * 16 * 1024;
        let prg_rom = &bytes[16..prg_rom_bytes + 16];
        let chr_rom_bytes = chr_rom_blocks as usize * 8 * 1024;
        let chr_rom = &bytes[prg_rom_bytes + 16..prg_rom_bytes + 16 + chr_rom_bytes];

        let mapper: Box<dyn Mapper> = match mapper_id {
            0 => Box::new(Mapper0::new(prg_rom, chr_rom, prg_rom_blocks, mirror_flag)?),
            1 => Box::new(Mapper1::new(prg_rom, chr_rom)?),
            4 => Box::new(Mapper4::new(prg_rom, chr_rom)?),
            id => return Err(format!("mapper {id} not implemented")),
        };

        Ok(Self {
            mapper,
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

    pub fn count_scanline(&mut self) {
        self.mapper.count_scanline();
        if self.mapper.check_irq() {
            self.bus.upgrade().unwrap().borrow_mut().request_irq();
        }
    }
}

#[derive(Debug)]
pub struct RomInfo {
    uses_nes_20: bool,
    prg_rom_blocks: u8,
    chr_rom_blocks: u8,
    has_persistent_prg_ram: bool,
    has_chr_ram: bool,
    mirror_flag: u8,
    uses_alternate_nametable_layout: bool,
    contains_trainer: bool,
    mapper_id: u8,
}

impl RomInfo {
    pub fn new(header: &[u8; 16]) -> Self {
        let uses_nes_20 = {
            let byte = header[7];
            !is_bit_set(byte, 2) && is_bit_set(byte, 3)
        };
        let prg_rom_blocks = header[4];
        let chr_rom_blocks = header[5];
        let has_persistent_prg_ram = header[6] & 0x02 != 0;
        let has_chr_ram = chr_rom_blocks == 0;
        let mirror_flag = header[6] & 0x01;
        let uses_alternate_nametable_layout = header[6] & 0x08 != 0;
        let contains_trainer = header[6] & 0x04 != 0;
        let mapper_id = header[6] >> 4 | (header[7] & 0xF0);

        Self {
            uses_nes_20,
            prg_rom_blocks,
            chr_rom_blocks,
            has_persistent_prg_ram,
            has_chr_ram,
            mirror_flag,
            uses_alternate_nametable_layout,
            contains_trainer,
            mapper_id,
        }
    }
}

impl std::fmt::Display for RomInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "uses nes 2.0 format: {}", self.uses_nes_20)?;
        writeln!(f, "prg rom size: {}k", self.prg_rom_blocks as usize * 16)?;
        writeln!(f, "chr rom size: {}k", self.chr_rom_blocks as usize * 8)?;
        writeln!(f, "has persistent prg ram: {}", self.has_persistent_prg_ram)?;
        writeln!(f, "has chr ram: {}", self.has_chr_ram)?;
        writeln!(
            f,
            "nametable layout (if hardwired): {}",
            if self.mirror_flag == 0 {
                "vertical"
            } else {
                "horizontal"
            }
        )?;
        writeln!(
            f,
            "uses alternate nametable layout: {}",
            self.uses_alternate_nametable_layout
        )?;
        writeln!(f, "contains trainer: {}", self.contains_trainer)?;
        write!(f, "mapper id: {}", self.mapper_id)?;

        Ok(())
    }
}
