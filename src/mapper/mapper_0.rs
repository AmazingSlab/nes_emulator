use super::{Mapper, Mirroring};

pub struct Mapper0 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    variant: NromVariant,
    mirror_flag: u8,
    has_chr_ram: bool,
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

        let has_chr_ram = chr_rom.is_empty();
        let chr_rom = if has_chr_ram {
            vec![0; 8 * 1024]
        } else {
            chr_rom.into()
        };

        Ok(Self {
            prg_rom: prg_rom.into(),
            chr_rom,
            variant,
            mirror_flag,
            has_chr_ram,
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

    fn cpu_write(&mut self, _addr: u16, _data: u8) {}

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
        if self.mirror_flag == 0 {
            Mirroring::Horizontal
        } else {
            Mirroring::Vertical
        }
    }
}
