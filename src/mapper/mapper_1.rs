use crate::{
    is_bit_set,
    savestate::{self, MapperState},
};

use super::{Mapper, Mirroring};

pub struct Mapper1 {
    prg_ram: Vec<u8>,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    has_chr_ram: bool,

    shift: u8,
    shift_count: u8,
    control: Control,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
    prg_banks: u8,
}

impl Mapper1 {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8]) -> Result<Self, String> {
        let prg_banks = (prg_rom.len() / (16 * 1024)) as u8;
        let has_chr_ram = chr_rom.is_empty();
        let chr_rom = if has_chr_ram {
            vec![0; 8 * 1024]
        } else {
            chr_rom.into()
        };

        Ok(Self {
            prg_ram: vec![0; 8 * 1024],
            prg_rom: prg_rom.into(),
            chr_rom,
            has_chr_ram,

            shift: 0,
            shift_count: 0,
            control: Control::default(),
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: prg_banks - 1,
            prg_banks,
        })
    }

    fn map_cpu_addr(&self, addr: u16) -> usize {
        let bank = match self.control.prg_bank_mode() {
            0 | 1 => self.prg_bank & 0x0E,
            2 => {
                if addr < 0xC000 {
                    0
                } else {
                    self.prg_bank & 0x0F
                }
            }
            3 => {
                if addr > 0xC000 {
                    self.prg_banks - 1
                } else {
                    self.prg_bank & 0x0F
                }
            }
            _ => unreachable!(),
        };

        let bank_size = if matches!(self.control.prg_bank_mode(), 0 | 1) {
            32
        } else {
            16
        };

        (addr as usize & (bank_size * 1024 - 1))
            | (bank as usize * 16 * 1024) & (self.prg_rom.len() - 1)
    }

    fn map_ppu_addr(&self, addr: u16) -> usize {
        let bank = if self.control.chr_bank_mode() == 0 {
            self.chr_bank_0 & 0x1E
        } else if addr < 0x1000 {
            self.chr_bank_0
        } else {
            self.chr_bank_1
        };

        let bank_size = if self.control.chr_bank_mode() == 0 {
            8
        } else {
            4
        };

        (addr as usize & (bank_size * 1024 - 1))
            | (bank as usize * 4 * 1024) & (self.chr_rom.len() - 1)
    }
}

impl Mapper for Mapper1 {
    fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[addr as usize & 0x1FFF],
            0x8000..=0xFFFF => {
                let addr = self.map_cpu_addr(addr);
                self.prg_rom[addr]
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[addr as usize & 0x1FFF] = data,
            0x8000..=0xFFFF => {
                if is_bit_set(data, 7) {
                    self.shift = 0;
                    self.shift_count = 0;
                } else if self.shift_count < 5 {
                    self.shift |= (data & 0x01) << 5;
                    self.shift >>= 1;
                    self.shift_count += 1;
                }
                if self.shift_count == 5 {
                    match addr {
                        0x8000..=0x9FFF => self.control.0 = self.shift,
                        0xA000..=0xBFFF => self.chr_bank_0 = self.shift,
                        0xC000..=0xDFFF => self.chr_bank_1 = self.shift,
                        0xE000..=0xFFFF => self.prg_bank = self.shift,
                        _ => unreachable!(),
                    }
                    self.shift = 0;
                    self.shift_count = 0;
                }
            }
            _ => (),
        }
    }

    fn ppu_read(&self, addr: u16) -> u8 {
        let addr = self.map_ppu_addr(addr);
        self.chr_rom[addr]
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        if self.has_chr_ram {
            let addr = self.map_ppu_addr(addr);
            self.chr_rom[addr] = data;
        }
    }

    fn mirroring(&self) -> super::Mirroring {
        match self.control.mirroring() {
            0 => Mirroring::SingleScreen,
            1 => Mirroring::SingleScreenUpper,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }

    fn apply_state(&mut self, state: MapperState) {
        for (description, section) in state {
            match description {
                "DREG" => {
                    [
                        self.control.0,
                        self.chr_bank_0,
                        self.chr_bank_1,
                        self.prg_bank,
                    ] = savestate::deserialize(section).unwrap_or_default()
                }
                "LRST" => {
                    // Internal timestamp used by FCEUX to determine if the mapper should accept a
                    // write.
                }
                "BFFR" => self.shift = savestate::deserialize(section).unwrap_or_default(),
                "BFRS" => self.shift_count = savestate::deserialize(section).unwrap_or_default(),
                "WRAM" => {
                    let Ok(prg_ram) = savestate::deserialize::<Vec<u8>>(section) else {
                        continue;
                    };
                    if prg_ram.len() == self.prg_ram.len() {
                        self.prg_ram = prg_ram;
                    }
                }
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
        use crate::savestate::serialize;

        let mut buffer = Vec::new();

        if self.has_chr_ram {
            buffer.extend_from_slice(&serialize(&self.chr_rom, "CHRR"));
        }

        buffer.extend_from_slice(&serialize(&self.prg_ram, "WRAM"));
        buffer.extend_from_slice(&serialize(
            &[
                self.control.0,
                self.chr_bank_0,
                self.chr_bank_1,
                self.prg_bank,
            ],
            "DREG",
        ));
        buffer.extend_from_slice(&serialize(&self.shift, "BFFR"));
        buffer.extend_from_slice(&serialize(&self.shift_count, "BFRS"));

        buffer
    }
}

#[bitfield_struct::bitfield(u8)]
#[derive(PartialEq, Eq)]
struct Control {
    #[bits(2)]
    mirroring: u8,
    #[bits(2)]
    prg_bank_mode: u8,
    #[bits(1)]
    chr_bank_mode: u8,
    #[bits(3)]
    __: u8,
}
