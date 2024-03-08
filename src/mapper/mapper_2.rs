use crate::savestate::{self, MapperState};

use super::{Mapper, Mirroring};

pub struct Mapper2 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    has_chr_ram: bool,

    prg_bank: u8,
    mirroring: Mirroring,
    prg_banks: u8,
}

impl Mapper2 {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8], mirror_flag: u8) -> Result<Self, String> {
        let has_chr_ram = chr_rom.is_empty();
        let chr_rom = if has_chr_ram {
            vec![0; 8 * 1024]
        } else {
            chr_rom.into()
        };

        let mirroring = if mirror_flag == 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        };

        Ok(Self {
            prg_rom: prg_rom.into(),
            chr_rom,
            has_chr_ram,
            mirroring,
            prg_bank: 0,
            prg_banks: (prg_rom.len() / (8 * 1024)) as u8,
        })
    }

    fn map_addr(&self, addr: u16) -> usize {
        let bank = match addr {
            0x8000..=0xBFFF => self.prg_bank,
            0xC000..=0xFFFF => self.prg_banks - 1,
            _ => 0,
        };

        (addr & 0x3FFF) as usize | (bank as usize * 16 * 1024) & (self.prg_rom.len() - 1)
    }
}

impl Mapper for Mapper2 {
    fn cpu_read(&self, addr: u16) -> u8 {
        let addr = self.map_addr(addr);
        self.prg_rom[addr]
    }

    fn cpu_write(&mut self, _addr: u16, data: u8) {
        self.prg_bank = data & 0x0F;
    }

    fn ppu_read(&self, addr: u16) -> u8 {
        let addr = addr as usize & 0x1FFF;
        self.chr_rom[addr]
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        if self.has_chr_ram {
            let addr = addr as usize & 0x1FFF;
            self.chr_rom[addr] = data;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn apply_state(&mut self, state: MapperState) {
        use savestate::deserialize;

        for (description, section) in state {
            match description {
                "BUSC" => (), // Bus conflict.
                "LATC" => self.prg_bank = deserialize(section).unwrap_or_default(),
                "CHRR" => {
                    if !self.has_chr_ram {
                        continue;
                    }
                    let Ok(chr_ram) = savestate::deserialize::<Vec<u8>>(section) else {
                        continue;
                    };
                    if chr_ram.len() == self.chr_rom.len() {
                        self.chr_rom = chr_ram;
                    }
                }
                _ => println!("warn: unrecognized section `{description}`"),
            }
        }
    }

    fn save_state(&self) -> Vec<u8> {
        use savestate::serialize;

        let mut buffer = Vec::new();

        if self.has_chr_ram {
            buffer.extend_from_slice(&serialize(&self.chr_rom, "CHRR"));
        }

        buffer.extend_from_slice(&serialize(&self.prg_bank, "LATC"));

        buffer
    }
}
