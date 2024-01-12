use crate::is_bit_set;

use super::{Mapper, Mirroring};

pub struct Mapper1 {
    prg_ram: Vec<u8>,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,

    shift: u8,
    shift_count: u8,
    control: u8,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
    prg_banks: u8,
}

impl Mapper1 {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8]) -> Result<Self, String> {
        Ok(Self {
            prg_ram: vec![0; 8 * 1024],
            prg_rom: prg_rom.into(),
            chr_rom: chr_rom.into(),

            shift: 0,
            shift_count: 0,
            control: 0,
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: 0,
            prg_banks: (prg_rom.len() / (16 * 1024)) as u8,
        })
    }

    fn map_cpu_addr(&self, addr: u16) -> usize {
        match (self.control >> 2) & 0x03 {
            0 | 1 => (addr & 0x7FFF) as usize | ((self.prg_bank & 0x0E) as usize * 32 * 1024),
            2 => {
                if addr < 0xC000 {
                    (addr & 0x3FFF) as usize | (16 * 1024)
                } else {
                    (addr & 0x3FFF) as usize | ((self.prg_bank & 0x0E) as usize * 16 * 1024)
                }
            }
            3 => {
                if addr > 0xC000 {
                    (addr & 0x3FFF) as usize | ((self.prg_banks - 1) as usize * 16 * 1024)
                } else {
                    (addr & 0x3FFF) as usize | ((self.prg_bank & 0x0E) as usize * 16 * 1024)
                }
            }
            _ => unreachable!(),
        }
    }

    fn map_ppu_addr(&self, addr: u16) -> usize {
        if self.control & (1 << 4) == 0 {
            (addr & 0x1FFF) as usize | ((self.chr_bank_0 & 0x1E) as usize * 8 * 1024)
        } else if addr < 0x1000 {
            (addr & 0x0FFF) as usize | ((self.chr_bank_0) as usize * 4 * 1024)
        } else {
            (addr & 0x0FFF) as usize | ((self.chr_bank_1) as usize * 4 * 1024)
        }
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
                        0x8000..=0x9FFF => self.control = self.shift,
                        0xA000..=0xBFFF => self.chr_bank_0 = self.shift,
                        0xC000..=0xDFFF => self.chr_bank_1 = self.shift,
                        0xE000..=0xFFFF => self.prg_bank = self.shift,
                        _ => (),
                    }
                    self.shift = 0;
                    self.shift_count = 0;
                }
            }
            _ => (),
        }
    }

    fn ppu_read(&self, addr: u16) -> u8 {
        if !self.chr_rom.is_empty() {
            let addr = self.map_ppu_addr(addr);
            self.chr_rom[addr]
        } else {
            0
        }
    }

    fn ppu_write(&mut self, _addr: u16, _data: u8) {}

    fn mirroring(&self) -> super::Mirroring {
        match self.control & 0x03 {
            0 => Mirroring::SingleScreen,
            1 => todo!(),
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }
}
