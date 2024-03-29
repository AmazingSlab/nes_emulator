use crate::savestate::{self, MapperState};

use super::{Mapper, Mirroring};

pub struct Mapper4 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    has_chr_ram: bool,

    bank_register: [u8; 8],
    bank_select: BankSelect,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload: bool,
    is_irq_enabled: bool,
    emit_irq: bool,
    mirroring: Mirroring,
    prg_ram_protect: u8,

    prg_banks: u8,
}

impl Mapper4 {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8]) -> Result<Self, String> {
        let has_chr_ram = chr_rom.is_empty();
        let chr_rom = if has_chr_ram {
            vec![0; 8 * 1024]
        } else {
            chr_rom.into()
        };

        Ok(Self {
            prg_rom: prg_rom.into(),
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            has_chr_ram,

            bank_register: [0; 8],
            bank_select: BankSelect::default(),
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            is_irq_enabled: false,
            emit_irq: false,
            mirroring: Mirroring::Vertical,
            prg_ram_protect: 0x80,

            prg_banks: (prg_rom.len() / (8 * 1024)) as u8,
        })
    }

    fn map_cpu_addr(&self, addr: u16) -> usize {
        let bank = match addr {
            0x8000..=0x9FFF => {
                if self.bank_select.prg_bank_mode() == 0 {
                    self.bank_register[6] & 0x3F
                } else {
                    self.prg_banks - 2
                }
            }
            0xA000..=0xBFFF => self.bank_register[7] & 0x3F,
            0xC000..=0xDFFF => {
                if self.bank_select.prg_bank_mode() != 0 {
                    self.bank_register[6] & 0x3F
                } else {
                    self.prg_banks - 2
                }
            }
            0xE000..=0xFFFF => self.prg_banks - 1,
            _ => 0,
        } & 0x03F;

        (addr & 0x1FFF) as usize | (bank as usize * 8 * 1024)
    }

    fn map_ppu_addr(&self, addr: u16) -> usize {
        let bank = if self.bank_select.chr_inversion() == 0 {
            match addr {
                0x0000..=0x07FF => self.bank_register[0] & 0xFE,
                0x0800..=0x0FFF => self.bank_register[1] & 0xFE,
                0x1000..=0x13FF => self.bank_register[2],
                0x1400..=0x17FF => self.bank_register[3],
                0x1800..=0x1BFF => self.bank_register[4],
                0x1C00..=0x1FFF => self.bank_register[5],
                _ => unreachable!(),
            }
        } else {
            match addr {
                0x0000..=0x03FF => self.bank_register[2],
                0x0400..=0x07FF => self.bank_register[3],
                0x0800..=0x0BFF => self.bank_register[4],
                0x0C00..=0x0FFF => self.bank_register[5],
                0x1000..=0x17FF => self.bank_register[0] & 0xFE,
                0x1800..=0x1FFF => self.bank_register[1] & 0xFE,
                _ => unreachable!(),
            }
        };

        let bank_size = if (self.bank_select.chr_inversion() == 0 && addr <= 0x0FFF)
            || (self.bank_select.chr_inversion() == 1 && addr >= 0x1000)
        {
            2
        } else {
            1
        };

        (addr as usize & (bank_size * 1024 - 1)) | (bank as usize * 1024) & (self.chr_rom.len() - 1)
    }
}

impl Mapper for Mapper4 {
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
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    self.bank_select.0 = data;
                } else {
                    self.bank_register[self.bank_select.bank_register() as usize] = data;
                }
            }
            0xA000..=0xBFFF => {
                if addr & 1 == 0 {
                    if data & 1 == 0 {
                        self.mirroring = Mirroring::Vertical;
                    } else {
                        self.mirroring = Mirroring::Horizontal;
                    }
                } else {
                    self.prg_ram_protect = data & 0xC0;
                }
            }
            0xC000..=0xDFFF => {
                if addr & 1 == 0 {
                    self.irq_latch = data;
                } else {
                    self.irq_reload = true;
                }
            }
            0xE000..=0xFFFF => {
                self.is_irq_enabled = addr & 1 != 0;
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

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn check_irq(&self) -> bool {
        self.emit_irq
    }

    fn count_scanline(&mut self) {
        self.emit_irq = false;

        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = self.irq_latch;
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }
        if self.irq_counter == 0 && self.is_irq_enabled {
            self.emit_irq = true;
        }
    }

    fn apply_state(&mut self, state: MapperState) {
        for (description, section) in state {
            match description {
                "REGS" => self.bank_register = savestate::deserialize(section).unwrap_or_default(),
                "CMD" => self.bank_select.0 = savestate::deserialize(section).unwrap_or_default(),
                "A000" => {
                    self.mirroring =
                        if savestate::deserialize::<u8>(section).unwrap_or_default() == 0 {
                            Mirroring::Vertical
                        } else {
                            Mirroring::Horizontal
                        }
                }
                "A001" => {
                    self.prg_ram_protect = savestate::deserialize(section).unwrap_or_default()
                }
                "IRQR" => self.irq_reload = savestate::deserialize(section).unwrap_or_default(),
                "IRQC" => self.irq_counter = savestate::deserialize(section).unwrap_or_default(),
                "IRQL" => self.irq_latch = savestate::deserialize(section).unwrap_or_default(),
                "IRQA" => self.is_irq_enabled = savestate::deserialize(section).unwrap_or_default(),
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
        buffer.extend_from_slice(&serialize(&self.bank_register, "REGS"));
        buffer.extend_from_slice(&serialize(&self.bank_select.0, "CMD"));
        buffer.extend_from_slice(&serialize(
            &match self.mirroring {
                Mirroring::Vertical => 0u8,
                Mirroring::Horizontal => 1u8,
                _ => unreachable!(),
            },
            "A000",
        ));
        buffer.extend_from_slice(&serialize(&self.prg_ram_protect, "A001"));
        buffer.extend_from_slice(&serialize(&self.irq_reload, "IRQR"));
        buffer.extend_from_slice(&serialize(&self.irq_counter, "IRQC"));
        buffer.extend_from_slice(&serialize(&self.irq_latch, "IRQL"));
        buffer.extend_from_slice(&serialize(&self.is_irq_enabled, "IRQA"));

        buffer
    }
}

#[bitfield_struct::bitfield(u8)]
#[derive(PartialEq, Eq)]
pub struct BankSelect {
    #[bits(3)]
    bank_register: u8,
    #[bits(3)]
    __: u8,
    #[bits(1)]
    prg_bank_mode: u8,
    #[bits(1)]
    chr_inversion: u8,
}
