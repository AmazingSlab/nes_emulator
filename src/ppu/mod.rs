use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use rand::Rng;

mod color;

use crate::Bus;
use color::Color;

#[derive(Debug)]
pub struct Ppu {
    control: u8,
    mask: u8,
    status: u8,
    ppu_addr: u16,

    bus: Weak<RefCell<Bus>>,
    pub buffer: [u8; 256 * 240 * 3],
    palette_ram: [u8; 32],
    cycle: u16,
    scanline: u16,
    addr_latch: u8,
    ppu_data_buffer: u8,

    pub is_frame_ready: bool,
    pub palette_number: u8,
    pub emit_nmi: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn connect_bus(&mut self, bus: Weak<RefCell<Bus>>) {
        self.bus = bus;
    }

    fn _bus(&self) -> Rc<RefCell<Bus>> {
        self.bus.upgrade().expect("bus not connected")
    }

    pub fn clock(&mut self) {
        // Draw random noise using a ROM's palettes.
        let pixel = rand::thread_rng().gen_range(0..4);
        let color_index = self.sample_palette_ram(self.palette_number, pixel);
        let color = Color::decode(color_index);
        self.draw_pixel(self.cycle, self.scanline, color);

        // NOTE: Not accurate.
        self.cycle += 1;
        if self.cycle == 1 && self.scanline == 241 {
            self.status |= 0x80;
            if self.control & 0x80 != 0 {
                self.emit_nmi = true;
            }
        }
        if self.cycle >= 341 {
            self.cycle = 0;
            self.scanline += 1;
        }
        if self.scanline >= 261 {
            self.scanline = 0;
            self.status &= 0x7F;
            self.is_frame_ready = true;
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
                self.ppu_data_buffer = self.ppu_read(self.ppu_addr);

                // The data delay applies to all memory locations except palette RAM.
                let data = if self.ppu_addr >= 0x3F00 {
                    self.ppu_data_buffer
                } else {
                    data
                };

                // Advance address horizontally/vertically depending on the control register.
                if self.control & (1 << 2) == 0 {
                    self.ppu_addr += 1;
                } else {
                    self.ppu_addr += 32;
                }
                data
            }
            _ => 0,
        }
    }

    /// Writes to the PPU's various registers. Accessible from the CPU.
    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x00 => self.control = data, // PPUCTRL.
            0x01 => self.mask = data,    // PPUMASK.
            0x02 => (),                  // PPUSTATUS; not writable.
            0x03 => (),                  // OAMADDR.
            0x04 => (),                  // OAMDATA.
            0x05 => (),                  // PPUSCROLL.
            // PPUADDR:
            0x06 => {
                // The CPU requires 2 writes to set the PPU's address.
                if self.addr_latch == 0 {
                    self.ppu_addr = (self.ppu_addr & 0x00FF) | ((data as u16) << 8);
                    self.addr_latch = 1;
                } else {
                    self.ppu_addr = (self.ppu_addr & 0xFF00) | data as u16;
                    self.addr_latch = 0;
                }
            }
            // PPUDATA.
            0x07 => {
                self.ppu_write(self.ppu_addr, data);

                // Advance address horizontally/vertically depending on the control register.
                if self.control & (1 << 2) == 0 {
                    self.ppu_addr += 1;
                } else {
                    self.ppu_addr += 32;
                }
            }
            _ => (),
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => 0, // Pattern tables.
            0x2000..=0x3EFF => 0, // Nametables.
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
            0x0000..=0x1FFF => (), // Pattern tables.
            0x2000..=0x3EFF => (), // Nametables.
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
            ppu_addr: 0,

            bus: Weak::new(),
            buffer: [0; 256 * 240 * 3],
            palette_ram: [0; 32],
            cycle: 0,
            scanline: 0,
            addr_latch: 0,
            ppu_data_buffer: 0,

            is_frame_ready: false,
            palette_number: 0,
            emit_nmi: false,
        }
    }
}
