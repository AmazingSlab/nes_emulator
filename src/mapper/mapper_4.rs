use super::{Mapper, Mirroring};

pub struct Mapper4 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    bank_register: [u8; 8],
    bank_select: BankSelect,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload: bool,
    is_irq_enabled: bool,
    emit_irq: bool,
    mirroring: Mirroring,

    prg_banks: u8,
}

impl Mapper4 {
    pub fn new(prg_rom: &[u8], chr_rom: &[u8]) -> Result<Self, String> {
        Ok(Self {
            prg_rom: prg_rom.into(),
            prg_ram: vec![0; 8 * 1024],
            chr_rom: chr_rom.into(),
            bank_register: [0; 8],
            bank_select: BankSelect::default(),
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            is_irq_enabled: false,
            emit_irq: false,
            mirroring: Mirroring::Vertical,

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
            _ => todo!(),
        } & 0x03F;

        (addr & 0x1FFF) as usize | (bank as usize * 8 * 1024)
    }

    fn map_ppu_addr(&self, addr: u16) -> usize {
        let bank = if self.bank_select.chr_inversion() == 0 {
            match addr {
                0x0000..=0x07FF => self.bank_register[0],
                0x0800..=0x0FFF => self.bank_register[1],
                0x1000..=0x13FF => self.bank_register[2],
                0x1400..=0x17FF => self.bank_register[3],
                0x1800..=0x1BFF => self.bank_register[4],
                0x1C00..=0x1FFF => self.bank_register[5],
                _ => todo!(),
            }
        } else {
            match addr {
                0x0000..=0x03FF => self.bank_register[2],
                0x0400..=0x07FF => self.bank_register[3],
                0x0800..=0x0BFF => self.bank_register[4],
                0x0C00..=0x0FFF => self.bank_register[5],
                0x1000..=0x17FF => self.bank_register[0],
                0x1800..=0x1FFF => self.bank_register[1],
                _ => todo!(),
            }
        };

        let bank_size = if (self.bank_select.chr_inversion() == 0 && addr <= 0x0FFF)
            || (self.bank_select.chr_inversion() == 1 && addr >= 0x1000)
        {
            2
        } else {
            1
        };

        (addr as usize & (bank_size * 1024 - 1)) | (bank as usize * 1024)
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
                    // PRG RAM protect.
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
        if !self.chr_rom.is_empty() {
            let addr = self.map_ppu_addr(addr);
            self.chr_rom[addr]
        } else {
            0
        }
    }

    fn ppu_write(&mut self, _addr: u16, _data: u8) {}

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
