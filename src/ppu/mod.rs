use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

mod color;

use crate::{cartridge::Mirroring, Bus};
use color::Color;

#[derive(Debug)]
pub struct Ppu {
    control: u8,
    mask: u8,
    status: u8,

    bus: Weak<RefCell<Bus>>,
    pub buffer: [u8; 256 * 240 * 3],
    nametables: [u8; 2048],
    palette_ram: [u8; 32],
    cycle: u16,
    scanline: u16,
    ppu_data_buffer: u8,
    vram_addr: u16,
    temp_vram_addr: u16,
    fine_x_scroll: u8,
    addr_latch: u8,

    pattern_table_shift_low: u16,
    pattern_table_shift_high: u16,
    palette_attrib_shift_low: u16,
    palette_attrib_shift_high: u16,

    next_tile_nametable: u8,
    next_tile_attrib: u8,
    next_tile_pattern_low: u8,
    next_tile_pattern_high: u8,

    pub is_frame_ready: bool,
    pub emit_nmi: bool,
    is_odd_frame: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn connect_bus(&mut self, bus: Weak<RefCell<Bus>>) {
        self.bus = bus;
    }

    fn bus(&self) -> Rc<RefCell<Bus>> {
        self.bus.upgrade().expect("bus not connected")
    }

    pub fn clock(&mut self) {
        if self.scanline <= 239 || self.scanline == 261 {
            if (self.cycle >= 2 && self.cycle <= 257) || (self.cycle >= 321 && self.cycle <= 337) {
                self.update_shift_registers();

                match (self.cycle - 1) % 8 {
                    0 => {
                        self.load_shift_registers();

                        self.next_tile_nametable =
                            self.ppu_read(0x2000 | (self.vram_addr & 0x0FFF));
                    }
                    2 => {
                        self.next_tile_attrib = self.ppu_read(
                            0x23C0
                                | (self.vram_addr & 0x0C00)
                                | (((self.vram_addr & 0x03E0) >> 7) << 3)
                                | ((self.vram_addr & 0x1F) >> 2),
                        );

                        if self.vram_addr & 0x40 != 0 {
                            self.next_tile_attrib >>= 4;
                        }
                        if self.vram_addr & 0x02 != 0 {
                            self.next_tile_attrib >>= 2;
                        }
                        self.next_tile_attrib &= 0x03;
                    }
                    4 => {
                        self.next_tile_pattern_low = self.ppu_read(
                            ((self.control as u16 & 0x10) << 8)
                                + ((self.next_tile_nametable as u16) << 4)
                                + (self.vram_addr >> 12),
                        );
                    }
                    6 => {
                        self.next_tile_pattern_high = self.ppu_read(
                            ((self.control as u16 & 0x10) << 8)
                                + ((self.next_tile_nametable as u16) << 4)
                                + (self.vram_addr >> 12)
                                + 8,
                        );
                    }
                    7 => {
                        self.increment_x_scroll();
                    }
                    _ => (),
                }
            }
            if self.cycle == 256 {
                self.increment_y_scroll();
            }
            if self.cycle == 257 {
                self.load_shift_registers();
                self.update_x_scroll();
            }
            if self.cycle == 338 || self.cycle == 340 {
                self.next_tile_nametable = self.ppu_read(0x2000 | self.vram_addr & 0x0FFF);
            }
        }
        if self.scanline == 240 {
            // Idle scanline; do nothing.
        }
        if self.cycle == 1 && self.scanline == 241 {
            self.status |= 0x80;
            if self.control & 0x80 != 0 {
                self.emit_nmi = true;
            }
        }
        if self.scanline == 261 {
            if self.cycle == 1 {
                self.status &= 0x7F;
                self.is_frame_ready = true;
                self.is_odd_frame = !self.is_odd_frame;
            }
            if self.cycle >= 280 && self.cycle <= 304 {
                self.update_y_scroll();
            }
            if self.cycle == 339 && self.is_odd_frame {
                self.cycle = 0;
                self.scanline = 0;
            }
            if self.cycle == 340 {
                self.cycle = 0;
                self.scanline = 0;
            }
        }
        let bit_mux = 0x8000 >> self.fine_x_scroll as u16;
        let pattern_low = ((self.pattern_table_shift_low & bit_mux) > 0) as u8;
        let pattern_high = ((self.pattern_table_shift_high & bit_mux) > 0) as u8;
        let attrib_low = ((self.palette_attrib_shift_low & bit_mux) > 0) as u8;
        let attrib_high = ((self.palette_attrib_shift_high & bit_mux) > 0) as u8;

        let palette = (attrib_high << 1) | attrib_low;
        let index = (pattern_high << 1) | pattern_low;
        let color_index = self.sample_palette_ram(palette, index);
        let color = Color::decode(color_index);

        self.draw_pixel(self.cycle - 1, self.scanline, color);
        if self.cycle == 340 {
            self.cycle = 0;
            self.scanline += 1;
        }
        self.cycle += 1;
    }

    fn update_x_scroll(&mut self) {
        if self.mask & 0x18 != 0 {
            self.vram_addr = (self.vram_addr & !0x041F) | (self.temp_vram_addr & 0x041F);
        }
    }

    fn update_y_scroll(&mut self) {
        if self.mask & 0x18 != 0 {
            self.vram_addr = (self.vram_addr & !0x7BE0) | (self.temp_vram_addr & 0x7BE0);
        }
    }

    fn increment_y_scroll(&mut self) {
        if self.mask & 0x18 != 0 {
            if self.vram_addr & 0x7000 != 0x7000 {
                self.vram_addr += 0x1000;
            } else {
                self.vram_addr &= !0x7000;
                let mut y = (self.vram_addr & 0x03E0) >> 5;
                if y == 29 {
                    y = 0;
                    self.vram_addr ^= 0x0800;
                } else if y == 31 {
                    y = 0;
                } else {
                    y += 1;
                }
                self.vram_addr = (self.vram_addr & !0x03E0) | (y << 5);
            }
        }
    }

    fn update_shift_registers(&mut self) {
        if self.mask & (1 << 3) != 0 {
            self.pattern_table_shift_low <<= 1;
            self.pattern_table_shift_high <<= 1;
            self.palette_attrib_shift_low <<= 1;
            self.palette_attrib_shift_high <<= 1;
        }
    }

    fn load_shift_registers(&mut self) {
        self.pattern_table_shift_low =
            (self.pattern_table_shift_low & 0xFF00) | self.next_tile_pattern_low as u16;
        self.pattern_table_shift_high =
            (self.pattern_table_shift_high & 0xFF00) | self.next_tile_pattern_high as u16;
        self.palette_attrib_shift_low = (self.palette_attrib_shift_low & 0xFF00)
            | if self.next_tile_attrib & 0b01 != 0 {
                0xFF
            } else {
                0x00
            };
        self.palette_attrib_shift_high = (self.palette_attrib_shift_high & 0xFF00)
            | if self.next_tile_attrib & 0b10 != 0 {
                0xFF
            } else {
                0x00
            };
    }

    fn increment_x_scroll(&mut self) {
        if self.mask & 0x18 != 0 {
            if self.vram_addr & 0x001F == 31 {
                self.vram_addr &= !0x001F;
                self.vram_addr ^= 0x0400;
            } else {
                self.vram_addr += 1;
            }
        }
    }

    /// Reads the PPU's various registers. Accessible from the CPU.
    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x00 => 0, // PPUCTRL; not readable.
            0x01 => 0, // PPUMASK; not readable.
            // PPUSTATUS.
            0x02 => {
                // Only the top 3 bits are meaningful. The other 5 contain stale PPU bus data.
                let data = (self.status & 0xE0) | (self.ppu_data_buffer & 0x1F);

                // VBlank flag is reset after reading.
                self.status &= 0x7F;

                // Reading PPUSTATUS resets the address latch.
                self.addr_latch = 0;

                data
            }
            0x03 => 0, // OAMADDR; not readable.
            0x04 => 0, // OAMDATA.
            0x05 => 0, // PPUSCROLL; not readable.
            0x06 => 0, // PPUADDR; not readable.
            // PPUDATA.
            0x07 => {
                // Data is delayed one read cycle. As such, the data returned is the data requested
                // the previous read.
                let data = self.ppu_data_buffer;
                self.ppu_data_buffer = self.ppu_read(self.vram_addr);

                // The data delay applies to all memory locations except palette RAM.
                let data = if self.vram_addr >= 0x3F00 {
                    self.ppu_data_buffer
                } else {
                    data
                };

                // Advance address horizontally/vertically depending on the control register.
                if self.control & (1 << 2) == 0 {
                    self.vram_addr += 1;
                } else {
                    self.vram_addr += 32;
                }
                data
            }
            _ => 0,
        }
    }

    /// Writes to the PPU's various registers. Accessible from the CPU.
    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            // PPUCTRL.
            0x00 => {
                self.control = data;
                // Set the nametable select bits of the VRAM address to the base nametable address
                // bits of the PPUCTRL register.
                self.temp_vram_addr = (self.temp_vram_addr & 0x73FF) | ((data as u16 & 0x03) << 10);
            }
            0x01 => self.mask = data, // PPUMASK.
            0x02 => (),               // PPUSTATUS; not writable.
            0x03 => (),               // OAMADDR.
            0x04 => (),               // OAMDATA.
            // PPUSCROLL.
            0x05 => {
                if self.addr_latch == 0 {
                    self.temp_vram_addr = (self.temp_vram_addr & 0x001F) | (data >> 3) as u16;
                    self.fine_x_scroll = data & 0x07;
                    self.addr_latch = 1;
                } else {
                    self.temp_vram_addr = (self.temp_vram_addr & 0x73E0)
                        | ((data as u16) << 12)
                        | ((data as u16 & 0xF8) << 2);
                    self.addr_latch = 0;
                }
            }
            // PPUADDR:
            0x06 => {
                // The CPU requires 2 writes to set the PPU's address.
                if self.addr_latch == 0 {
                    self.temp_vram_addr =
                        (self.temp_vram_addr & 0x00FF) | ((data as u16 & 0x3F) << 8);
                    self.addr_latch = 1;
                } else {
                    self.temp_vram_addr = (self.temp_vram_addr & 0xFF00) | data as u16;
                    self.vram_addr = self.temp_vram_addr;
                    self.addr_latch = 0;
                }
            }
            // PPUDATA.
            0x07 => {
                self.ppu_write(self.vram_addr, data);

                // Advance address horizontally/vertically depending on the control register.
                if self.control & (1 << 2) == 0 {
                    self.vram_addr += 1;
                } else {
                    self.vram_addr += 32;
                }
            }
            _ => (),
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.bus().borrow().ppu_read(addr),
            0x2000..=0x3EFF => {
                // TODO: Get from cartridge.
                let mirroring = Mirroring::Vertical;
                match mirroring {
                    Mirroring::Horizontal => todo!(),
                    Mirroring::Vertical => self.nametables[addr as usize & 0x07FF],
                }
            }
            // Palette RAM.
            0x3F00..=0x3FFF => {
                let addr = addr & 0x1F;

                // Addresses 0x04, 0x08, 0x0C (transparent colors of background palettes) can
                // contain data not normally used by the PPU for rendering, but 0x10, 0x14, 0x18,
                // 0x1C (transparent colors of sprite palettes) are mirrors of 0x00, 0x04, 0x08,
                // 0x0C, respectively.
                let addr = match addr {
                    0x10 => 0x00,
                    0x14 => 0x04,
                    0x18 => 0x08,
                    0x1C => 0x0C,
                    _ => addr,
                };
                self.palette_ram[addr as usize]
            }
            _ => 0,
        }
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        match addr {
            // 0x0000..=0x1FFF => self.bus().borrow_mut().ppu_write(addr, data),
            0x0000..=0x1FFF => (),
            0x2000..=0x3EFF => {
                // TODO: Get from cartridge.
                let mirroring = Mirroring::Vertical;
                match mirroring {
                    Mirroring::Horizontal => todo!(),
                    Mirroring::Vertical => self.nametables[addr as usize & 0x07FF] = data,
                }
            }
            // Palette RAM.
            0x3F00..=0x3FFF => {
                let addr = addr & 0x1F;

                // Addresses 0x04, 0x08, 0x0C (transparent colors of background palettes) can
                // contain data not normally used by the PPU for rendering, but 0x10, 0x14, 0x18,
                // 0x1C (transparent colors of sprite palettes) are mirrors of 0x00, 0x04, 0x08,
                // 0x0C, respectively.
                let addr = match addr {
                    0x10 => 0x00,
                    0x14 => 0x04,
                    0x18 => 0x08,
                    0x1C => 0x0C,
                    _ => addr,
                };
                self.palette_ram[addr as usize] = data;
            }
            _ => (),
        }
    }

    fn draw_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x >= 256 || y >= 240 {
            return;
        }
        let index = (x + y * 256) as usize;
        self.buffer[index * 3] = color.r;
        self.buffer[index * 3 + 1] = color.g;
        self.buffer[index * 3 + 2] = color.b;
    }

    fn sample_palette_ram(&self, palette: u8, index: u8) -> u8 {
        self.ppu_read(0x3F00 + ((palette << 2) + index) as u16)
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            control: 0,
            mask: 0,
            status: 0,

            bus: Weak::new(),
            buffer: [0; 256 * 240 * 3],
            nametables: [0; 2048],
            palette_ram: [0; 32],
            cycle: 0,
            scanline: 0,
            ppu_data_buffer: 0,
            vram_addr: 0,
            temp_vram_addr: 0,
            fine_x_scroll: 0,
            addr_latch: 0,

            pattern_table_shift_low: 0,
            pattern_table_shift_high: 0,
            palette_attrib_shift_low: 0,
            palette_attrib_shift_high: 0,

            next_tile_nametable: 0,
            next_tile_attrib: 0,
            next_tile_pattern_low: 0,
            next_tile_pattern_high: 0,

            is_frame_ready: false,
            emit_nmi: false,
            is_odd_frame: false,
        }
    }
}
