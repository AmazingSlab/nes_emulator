use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

mod color;

use crate::{mapper::Mirroring, Bus, Cartridge};
use color::Color;

pub struct Ppu {
    control: PpuControl,
    mask: PpuMask,
    status: PpuStatus,

    bus: Weak<RefCell<Bus>>,
    cartridge: Rc<RefCell<Cartridge>>,
    #[cfg(not(feature = "wasm"))]
    buffer: Box<[u8; 256 * 240 * 3]>,
    #[cfg(feature = "wasm")]
    buffer: Box<[u8; 256 * 240 * 4]>,
    #[cfg(feature = "memview")]
    nametable_buffer: Box<[u8; 512 * 480 * 3]>,
    #[cfg(feature = "memview")]
    pattern_table_buffer: Box<[u8; 256 * 128 * 3]>,
    #[cfg(feature = "memview")]
    oam_buffer: Box<[u8; 64 * 64 * 3]>,
    nametables: [u8; 2048],
    palette_ram: [u8; 32],
    oam: [u8; 256],
    pub oam_addr: u8,
    pub oam_dma_page: u8,
    cycle: u16,
    scanline: u16,
    ppu_data_buffer: u8,
    vram_addr: VramAddress,
    temp_vram_addr: VramAddress,
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

    secondary_oam: [u8; 32],
    secondary_oam_sprite_count: u8,
    sprite_pattern_shift_low: [u8; 8],
    sprite_pattern_shift_high: [u8; 8],
    sprite_attrib: [u8; 8],
    sprite_x_pos: [u8; 8],

    pub is_frame_ready: bool,
    pub emit_nmi: bool,
    pub palette: u8,
    is_odd_frame: bool,
}

impl Ppu {
    pub fn new(cartridge: Rc<RefCell<Cartridge>>) -> Self {
        // Allocate directly on the heap without going through the stack.
        // This is necessary to avoid stack overflows in debug builds without having to sacrifice
        // the array length guarantee, as without optimizations, Box::new([T; N]) allocates the
        // array on the stack before moving to the heap.
        //
        // SAFETY: A raw pointer to memory previously owned by a Box is always safe to turn back
        // into a Box. Casting to a fixed-size array pointer is safe because the Vec is guaranteed
        // to have the same number of elements.
        #[cfg(not(feature = "wasm"))]
        let buffer = unsafe {
            Box::from_raw(Box::into_raw(vec![0u8; 256 * 240 * 3].into_boxed_slice())
                as *mut [u8; 256 * 240 * 3])
        };
        #[cfg(feature = "wasm")]
        let buffer = unsafe {
            Box::from_raw(Box::into_raw(vec![0u8; 256 * 240 * 4].into_boxed_slice())
                as *mut [u8; 256 * 240 * 4])
        };
        #[cfg(feature = "memview")]
        let nametable_buffer = unsafe {
            Box::from_raw(Box::into_raw(vec![0u8; 512 * 480 * 3].into_boxed_slice())
                as *mut [u8; 512 * 480 * 3])
        };
        #[cfg(feature = "memview")]
        let pattern_table_buffer = unsafe {
            Box::from_raw(Box::into_raw(vec![0u8; 256 * 128 * 3].into_boxed_slice())
                as *mut [u8; 256 * 128 * 3])
        };
        #[cfg(feature = "memview")]
        let oam_buffer = unsafe {
            Box::from_raw(
                Box::into_raw(vec![0u8; 64 * 64 * 3].into_boxed_slice()) as *mut [u8; 64 * 64 * 3]
            )
        };

        Self {
            control: PpuControl::default(),
            mask: PpuMask::default(),
            status: PpuStatus::default(),

            bus: Weak::new(),
            cartridge,
            buffer,
            #[cfg(feature = "memview")]
            nametable_buffer,
            #[cfg(feature = "memview")]
            pattern_table_buffer,
            #[cfg(feature = "memview")]
            oam_buffer,
            nametables: [0; 2048],
            palette_ram: [0; 32],
            oam: [0; 256],
            oam_addr: 0,
            oam_dma_page: 0,
            cycle: 0,
            scanline: 0,
            ppu_data_buffer: 0,
            vram_addr: VramAddress::default(),
            temp_vram_addr: VramAddress::default(),
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

            secondary_oam: [0; 32],
            secondary_oam_sprite_count: 0,
            sprite_pattern_shift_low: [0; 8],
            sprite_pattern_shift_high: [0; 8],
            sprite_attrib: [0; 8],
            sprite_x_pos: [0; 8],

            is_frame_ready: false,
            emit_nmi: false,
            palette: 0,
            is_odd_frame: false,
        }
    }

    pub fn reset(&mut self) {
        self.control = PpuControl::default();
        self.mask = PpuMask::default();
        self.status = PpuStatus::default();

        self.cycle = 0;
        self.scanline = 0;
        self.ppu_data_buffer = 0;
        self.fine_x_scroll = 0;
        self.addr_latch = 0;

        self.pattern_table_shift_low = 0;
        self.pattern_table_shift_high = 0;
        self.palette_attrib_shift_low = 0;
        self.palette_attrib_shift_high = 0;

        self.next_tile_nametable = 0;
        self.next_tile_attrib = 0;
        self.next_tile_pattern_low = 0;
        self.next_tile_pattern_high = 0;

        self.is_frame_ready = false;
        self.emit_nmi = false;
        self.is_odd_frame = false;
    }

    pub fn connect_bus(&mut self, bus: Weak<RefCell<Bus>>) {
        self.bus = bus;
    }

    #[cfg(not(feature = "wasm"))]
    pub fn buffer(&self) -> &[u8] {
        self.buffer.as_ref()
    }

    #[cfg(feature = "wasm")]
    pub fn buffer_raw(&self) -> *const u8 {
        self.buffer.as_ptr()
    }

    #[cfg(feature = "memview")]
    pub fn nametable_buffer(&self) -> &[u8] {
        self.nametable_buffer.as_ref()
    }

    #[cfg(feature = "memview")]
    pub fn pattern_table_buffer(&self) -> &[u8] {
        self.pattern_table_buffer.as_ref()
    }

    #[cfg(feature = "memview")]
    pub fn oam_buffer(&self) -> &[u8] {
        self.oam_buffer.as_ref()
    }

    pub fn clock(&mut self) {
        if self.scanline <= 239 || self.scanline == 261 {
            if self.cycle >= 2 && self.cycle <= 257 && self.mask.show_sprites() {
                for i in 0..8 {
                    if self.sprite_x_pos[i] != 0 {
                        self.sprite_x_pos[i] -= 1;
                    } else {
                        self.sprite_pattern_shift_low[i] <<= 1;
                        self.sprite_pattern_shift_high[i] <<= 1;
                    }
                }
            }
            if (self.cycle >= 2 && self.cycle <= 257) || (self.cycle >= 321 && self.cycle <= 337) {
                self.update_shift_registers();

                match (self.cycle - 1) % 8 {
                    0 => {
                        self.load_shift_registers();

                        self.next_tile_nametable =
                            self.ppu_read(0x2000 | (self.vram_addr.0 & 0x0FFF));
                    }
                    2 => {
                        self.next_tile_attrib = self.ppu_read(
                            0x23C0
                                | (self.vram_addr.nametable_y() << 11)
                                | (self.vram_addr.nametable_x() << 10)
                                | ((self.vram_addr.coarse_y() >> 2) << 3)
                                | (self.vram_addr.coarse_x() >> 2),
                        );

                        if self.vram_addr.coarse_y() & 0x02 != 0 {
                            self.next_tile_attrib >>= 4;
                        }
                        if self.vram_addr.coarse_x() & 0x02 != 0 {
                            self.next_tile_attrib >>= 2;
                        }
                        self.next_tile_attrib &= 0x03;
                    }
                    4 => {
                        self.next_tile_pattern_low = self.ppu_read(
                            ((self.control.background_pattern() as u16) << 12)
                                + ((self.next_tile_nametable as u16) << 4)
                                + self.vram_addr.fine_y(),
                        );
                    }
                    6 => {
                        self.next_tile_pattern_high = self.ppu_read(
                            ((self.control.background_pattern() as u16) << 12)
                                + ((self.next_tile_nametable as u16) << 4)
                                + self.vram_addr.fine_y()
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
            if self.cycle == 260 && (self.mask.show_background() || self.mask.show_sprites()) {
                self.cartridge.borrow_mut().count_scanline();
            }
            if self.cycle == 338 || self.cycle == 340 {
                self.next_tile_nametable = self.ppu_read(0x2000 | self.vram_addr.0 & 0x0FFF);
            }
        }
        if self.scanline == 240 {
            // Idle scanline; do nothing.
        }
        if self.cycle == 1 && self.scanline == 241 {
            self.status.set_vblank(true);
            if self.control.nmi() {
                self.emit_nmi = true;
            }
        }
        if self.scanline == 261 {
            if self.cycle == 1 {
                self.status.set_vblank(false);
                self.status.set_sprite_zero_hit(false);
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
        if self.scanline <= 239 {
            if self.cycle == 64 {
                self.secondary_oam = [0xFF; 32];
                self.secondary_oam_sprite_count = 0;
            }
            if self.cycle == 257 {
                for sprite in 0..64 {
                    let y_pos = self.oam[sprite * 4];
                    if self.scanline.wrapping_sub(y_pos as u16)
                        < (self.control.sprite_size() as u16 + 1) * 8
                    {
                        for i in 0..4 {
                            self.secondary_oam[self.secondary_oam_sprite_count as usize * 4 + i] =
                                self.oam[sprite * 4 + i];
                        }
                        self.secondary_oam_sprite_count += 1;
                        if self.secondary_oam_sprite_count == 8 {
                            break;
                        }
                    }
                }
            }
            if self.cycle == 320 {
                for i in 0..self.secondary_oam_sprite_count as usize {
                    self.sprite_x_pos[i] = self.secondary_oam[i * 4 + 3];
                    let y_pos = self.secondary_oam[i * 4];
                    let index = self.secondary_oam[i * 4 + 1];
                    let attrib = self.secondary_oam[i * 4 + 2];
                    let flip_horizontally = attrib & (1 << 6) != 0;
                    let flip_vertically = attrib & (1 << 7) != 0;
                    let line = (self.scanline.wrapping_sub(y_pos as u16)) & 0x0F;

                    let pattern_low;
                    let pattern_high;
                    if self.control.sprite_size() == 0 {
                        let line = line & 0x07;
                        let line = if flip_vertically { 7 - line } else { line };
                        pattern_low = self.ppu_read(
                            ((self.control.sprite_pattern() as u16) << 12)
                                | ((index as u16) << 4)
                                | line,
                        );
                        pattern_high = self.ppu_read(
                            ((self.control.sprite_pattern() as u16) << 12)
                                | ((index as u16) << 4)
                                | 8
                                | line,
                        );
                    } else if (line < 8 && !flip_vertically) || (flip_vertically && line > 7) {
                        let line = line & 0x07;
                        let line = if flip_vertically { 7 - line } else { line };
                        pattern_low = self.ppu_read(
                            ((index as u16 & 1) << 12) | ((index as u16 & 0xFE) << 4) | line,
                        );
                        pattern_high = self.ppu_read(
                            ((index as u16 & 1) << 12) | ((index as u16 & 0xFE) << 4) | 8 | line,
                        );
                    } else {
                        let line = line & 0x07;
                        let line = if flip_vertically { 7 - line } else { line };
                        pattern_low = self.ppu_read(
                            ((index as u16 & 1) << 12) | (((index as u16 & 0xFE) + 1) << 4) | line,
                        );
                        pattern_high = self.ppu_read(
                            ((index as u16 & 1) << 12)
                                | (((index as u16 & 0xFE) + 1) << 4)
                                | 8
                                | line,
                        );
                    }
                    let (pattern_low, pattern_high) = if flip_horizontally {
                        (pattern_low.reverse_bits(), pattern_high.reverse_bits())
                    } else {
                        (pattern_low, pattern_high)
                    };
                    self.sprite_pattern_shift_low[i] = pattern_low;
                    self.sprite_pattern_shift_high[i] = pattern_high;
                    self.sprite_attrib[i] = attrib;
                }
            }
        }

        let bit_mux = 0x8000 >> self.fine_x_scroll as u16;
        let background_pattern_low = ((self.pattern_table_shift_low & bit_mux) > 0) as u8;
        let background_pattern_high = ((self.pattern_table_shift_high & bit_mux) > 0) as u8;
        let background_attrib_low = ((self.palette_attrib_shift_low & bit_mux) > 0) as u8;
        let background_attrib_high = ((self.palette_attrib_shift_high & bit_mux) > 0) as u8;

        let background_palette = (background_attrib_high << 1) | background_attrib_low;
        let background_index = (background_pattern_high << 1) | background_pattern_low;

        let mut sprite_pattern = 0;
        let mut sprite_palette = 0;
        let mut sprite_attrib = 0;
        for sprite in 0..8 {
            if self.sprite_x_pos[sprite] != 0 {
                continue;
            }
            let pattern_low = (self.sprite_pattern_shift_low[sprite] & 0x80 > 0) as u8;
            let pattern_high = (self.sprite_pattern_shift_high[sprite] & 0x80 > 0) as u8;
            let pattern = (pattern_high << 1) | pattern_low;
            if pattern != 0 {
                sprite_pattern = pattern;
                let attrib = self.sprite_attrib[sprite];
                sprite_palette = attrib & 0x03;
                sprite_attrib = attrib;
                break;
            }
        }
        let sprite_pattern = sprite_pattern;
        let sprite_palette = sprite_palette;

        let mut color_index = 0;
        if background_index == 0 && sprite_pattern != 0 {
            color_index = self.sample_palette_ram(sprite_palette + 4, sprite_pattern);
        } else if background_index != 0 && sprite_pattern == 0 {
            color_index = self.sample_palette_ram(background_palette, background_index);
        } else if background_index != 0 && sprite_pattern != 0 {
            self.status.set_sprite_zero_hit(true);
            if sprite_attrib & (1 << 5) == 0 {
                color_index = self.sample_palette_ram(sprite_palette + 4, sprite_pattern);
            } else {
                color_index = self.sample_palette_ram(background_palette, background_index);
            }
        } else if background_index == 0 && sprite_palette == 0 {
            color_index = self.sample_palette_ram(0, 0);
        }

        let color = Color::decode(color_index);

        self.draw_pixel(self.cycle.saturating_sub(1), self.scanline, color);
        if self.cycle == 340 {
            self.cycle = 0;
            self.scanline += 1;
        }
        self.cycle += 1;
    }

    fn update_x_scroll(&mut self) {
        if self.mask.show_background() || self.mask.show_sprites() {
            self.vram_addr
                .set_nametable_x(self.temp_vram_addr.nametable_x());
            self.vram_addr.set_coarse_x(self.temp_vram_addr.coarse_x());
        }
    }

    fn update_y_scroll(&mut self) {
        if self.mask.show_background() || self.mask.show_sprites() {
            self.vram_addr
                .set_nametable_y(self.temp_vram_addr.nametable_y());
            self.vram_addr.set_coarse_y(self.temp_vram_addr.coarse_y());
            self.vram_addr.set_fine_y(self.temp_vram_addr.fine_y());
        }
    }

    fn increment_y_scroll(&mut self) {
        if self.mask.show_background() || self.mask.show_sprites() {
            if self.vram_addr.fine_y() < 7 {
                self.vram_addr.set_fine_y(self.vram_addr.fine_y() + 1);
            } else {
                self.vram_addr.set_fine_y(0);
                let mut y = self.vram_addr.coarse_y();
                if y == 29 {
                    y = 0;
                    self.vram_addr
                        .set_nametable_y(self.vram_addr.nametable_y() ^ 1);
                } else if y == 31 {
                    y = 0;
                } else {
                    y += 1;
                }
                self.vram_addr.set_coarse_y(y);
            }
        }
    }

    fn update_shift_registers(&mut self) {
        if self.mask.show_background() {
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
        if self.mask.show_background() || self.mask.show_sprites() {
            if self.vram_addr.coarse_x() == 31 {
                self.vram_addr.set_coarse_x(0);
                self.vram_addr
                    .set_nametable_x(self.vram_addr.nametable_x() ^ 1);
            } else {
                self.vram_addr.set_coarse_x(self.vram_addr.coarse_x() + 1);
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
                let data = (self.status.0 & 0xE0) | (self.ppu_data_buffer & 0x1F);
                self.status.set_vblank(false);
                self.addr_latch = 0;

                data
            }
            0x03 => 0,                                // OAMADDR; not readable.
            0x04 => self.oam[self.oam_addr as usize], // OAMDATA.
            0x05 => 0,                                // PPUSCROLL; not readable.
            0x06 => 0,                                // PPUADDR; not readable.
            // PPUDATA.
            0x07 => {
                // Data is delayed one read cycle. As such, the data returned is the data requested
                // the previous read.
                let data = self.ppu_data_buffer;
                self.ppu_data_buffer = self.ppu_read(self.vram_addr.0);

                // The data delay applies to all memory locations except palette RAM.
                let data = if self.vram_addr.0 >= 0x3F00 {
                    self.ppu_data_buffer
                } else {
                    data
                };

                // Advance address horizontally/vertically depending on the control register.
                if self.control.address_increment() == 0 {
                    self.vram_addr.0 += 1;
                } else {
                    self.vram_addr.0 += 32;
                }
                data
            }
            0x4014 => 0, // OAMDMA; not readable.
            _ => 0,
        }
    }

    /// Writes to the PPU's various registers. Accessible from the CPU.
    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            // PPUCTRL.
            0x00 => {
                self.control.0 = data;
                self.temp_vram_addr.set_nametable_x(data as u16 & 0b01);
                self.temp_vram_addr
                    .set_nametable_y((data as u16 & 0b10) >> 1);
            }
            0x01 => self.mask.0 = data,   // PPUMASK.
            0x02 => (),                   // PPUSTATUS; not writable.
            0x03 => self.oam_addr = data, // OAMADDR.
            // OAMDATA.
            0x04 => {
                self.oam[self.oam_addr as usize] = data;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            // PPUSCROLL.
            0x05 => {
                if self.addr_latch == 0 {
                    self.temp_vram_addr.set_coarse_x(data as u16 >> 3);
                    self.fine_x_scroll = data & 0x07;
                    self.addr_latch = 1;
                } else {
                    self.temp_vram_addr.set_coarse_y(data as u16 >> 3);
                    self.temp_vram_addr.set_fine_y(data as u16 & 0x07);
                    self.addr_latch = 0;
                }
            }
            // PPUADDR:
            0x06 => {
                // The CPU requires 2 writes to set the PPU's address.
                if self.addr_latch == 0 {
                    self.temp_vram_addr.0 =
                        (self.temp_vram_addr.0 & !0xFF00) | ((data as u16 & 0x3F) << 8);
                    self.addr_latch = 1;
                } else {
                    self.temp_vram_addr.0 = (self.temp_vram_addr.0 & !0x00FF) | data as u16;
                    self.vram_addr = self.temp_vram_addr;
                    self.addr_latch = 0;
                }
            }
            // PPUDATA.
            0x07 => {
                self.ppu_write(self.vram_addr.0, data);

                // Advance address horizontally/vertically depending on the control register.
                if self.control.address_increment() == 0 {
                    self.vram_addr.0 += 1;
                } else {
                    self.vram_addr.0 += 32;
                }
            }
            0x4014 => self.oam_dma_page = data, // OAMDMA.
            _ => (),
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cartridge.borrow().ppu_read(addr),
            0x2000..=0x3EFF => {
                let mirroring = self.cartridge.borrow().mirroring();
                match mirroring {
                    Mirroring::Horizontal => {
                        let addr = addr & 0x0FFF;
                        if addr < 0x0800 {
                            self.nametables[addr as usize & 0x03FF]
                        } else {
                            self.nametables[(addr as usize & 0x03FF) + 0x0400]
                        }
                    }
                    Mirroring::Vertical => self.nametables[addr as usize & 0x07FF],
                    Mirroring::SingleScreen => self.nametables[addr as usize & 0x03FF],
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
            0x0000..=0x1FFF => self.cartridge.borrow_mut().ppu_write(addr, data),
            0x2000..=0x3EFF => {
                let mirroring = self.cartridge.borrow().mirroring();
                match mirroring {
                    Mirroring::Horizontal => {
                        let addr = addr & 0x0FFF;
                        if addr < 0x0800 {
                            self.nametables[addr as usize & 0x03FF] = data;
                        } else {
                            self.nametables[(addr as usize & 0x03FF) + 0x0400] = data;
                        }
                    }
                    Mirroring::Vertical => self.nametables[addr as usize & 0x07FF] = data,
                    Mirroring::SingleScreen => self.nametables[addr as usize & 0x03FF] = data,
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

    #[cfg(feature = "memview")]
    pub fn draw_nametables(&mut self) {
        for nametable_y in 0..=1 {
            for nametable_x in 0..=1 {
                for tile_y in 0..30 {
                    for tile_x in 0..32 {
                        let nametable = self.ppu_read(
                            0x2000
                                | (nametable_y << 11)
                                | (nametable_x << 10)
                                | (tile_y << 5)
                                | tile_x,
                        );
                        let mut attrib = self.ppu_read(
                            0x23C0
                                | (nametable_y << 11)
                                | (nametable_x << 10)
                                | ((tile_y >> 2) << 3)
                                | (tile_x >> 2),
                        );
                        if tile_y & 0x02 != 0 {
                            attrib >>= 4
                        }
                        if tile_x & 0x02 != 0 {
                            attrib >>= 2
                        }
                        let attrib = attrib & 0x03;
                        let background_pattern = (self.control.background_pattern() as u16) << 12;
                        let mut pattern_low = [0u8; 8];
                        for i in 0..8 {
                            let value =
                                self.ppu_read(background_pattern + ((nametable as u16) << 4) + i);
                            pattern_low[i as usize] = value;
                        }
                        let mut pattern_high = [0u8; 8];
                        for i in 0..8 {
                            let value = self
                                .ppu_read(background_pattern + ((nametable as u16) << 4) + i + 8);
                            pattern_high[i as usize] = value;
                        }

                        for (y, (low, high)) in pattern_low
                            .into_iter()
                            .zip(pattern_high.into_iter())
                            .enumerate()
                        {
                            for x in 0..8 {
                                let low = (low & (0x80 >> x) > 0) as u8;
                                let high = (high & (0x80 >> x) > 0) as u8;
                                let index = (high << 1) | low;
                                let color_index = if index != 0 {
                                    self.sample_palette_ram(attrib, index)
                                } else {
                                    self.sample_palette_ram(0, 0)
                                };
                                let color = Color::decode(color_index);

                                let index = x
                                    + tile_x as usize * 8
                                    + nametable_x as usize * 256
                                    + (y + tile_y as usize * 8 + nametable_y as usize * 240) * 512;
                                self.nametable_buffer[index * 3] = color.r;
                                self.nametable_buffer[index * 3 + 1] = color.g;
                                self.nametable_buffer[index * 3 + 2] = color.b;
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "memview")]
    pub fn draw_pattern_tables(&mut self) {
        for table_half in 0..=1 {
            for tile_y in 0..16 {
                for tile_x in 0..16 {
                    let mut pattern_low = [0u8; 8];
                    for i in 0..8 {
                        let value =
                            self.ppu_read((table_half << 12) | (tile_y << 8) | (tile_x << 4) | i);
                        pattern_low[i as usize] = value;
                    }
                    let mut pattern_high = [0u8; 8];
                    for i in 0..8 {
                        let value = self
                            .ppu_read((table_half << 12) | (tile_y << 8) | (tile_x << 4) | i | 8);
                        pattern_high[i as usize] = value;
                    }

                    for (y, (low, high)) in pattern_low
                        .into_iter()
                        .zip(pattern_high.into_iter())
                        .enumerate()
                    {
                        for x in 0..8 {
                            let low = (low & (0x80 >> x) > 0) as u8;
                            let high = (high & (0x80 >> x) > 0) as u8;
                            let index = (high << 1) | low;
                            let palette = if self.control.background_pattern() == table_half as u8 {
                                self.palette
                            } else {
                                self.palette + 4
                            };
                            let color_index = self.sample_palette_ram(palette, index);
                            let color = Color::decode(color_index);

                            let index = x
                                + tile_x as usize * 8
                                + table_half as usize * 128
                                + (y + tile_y as usize * 8) * 256;
                            self.pattern_table_buffer[index * 3] = color.r;
                            self.pattern_table_buffer[index * 3 + 1] = color.g;
                            self.pattern_table_buffer[index * 3 + 2] = color.b;
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "memview")]
    pub fn draw_oam(&mut self) {
        for sprite in 0..64 {
            let index = self.oam[sprite as usize * 4 + 1] as u16;
            let attrib = self.oam[sprite as usize * 4 + 2];
            let palette = attrib & 0x03;
            let flip_horizontally = attrib & (1 << 6) != 0;
            let flip_vertically = attrib & (1 << 7) != 0;

            let mut pattern_low = [0u8; 8];
            for i in 0..8 {
                let value = self
                    .ppu_read(((self.control.sprite_pattern() as u16) << 12) | (index << 4) | i);
                pattern_low[i as usize] = if flip_horizontally {
                    value.reverse_bits()
                } else {
                    value
                };
            }
            let mut pattern_high = [0u8; 8];
            for i in 0..8 {
                let value = self.ppu_read(
                    ((self.control.sprite_pattern() as u16) << 12) | (index << 4) | i | 8,
                );
                pattern_high[i as usize] = if flip_horizontally {
                    value.reverse_bits()
                } else {
                    value
                };
            }

            let sprite_x = sprite & 0x07;
            let sprite_y = sprite >> 3;

            for (y, (low, high)) in pattern_low
                .into_iter()
                .zip(pattern_high.into_iter())
                .enumerate()
            {
                let y = if flip_vertically { 7 - y } else { y };
                for x in 0..8 {
                    let low = (low & (0x80 >> x) > 0) as u8;
                    let high = (high & (0x80 >> x) > 0) as u8;
                    let index = (high << 1) | low;
                    let color_index = self.sample_palette_ram(palette + 4, index);
                    let color = Color::decode(color_index);

                    let index = x + sprite_x as usize * 8 + (y + sprite_y as usize * 8) * 64;
                    self.oam_buffer[index * 3] = color.r;
                    self.oam_buffer[index * 3 + 1] = color.g;
                    self.oam_buffer[index * 3 + 2] = color.b;
                }
            }
        }
    }

    #[cfg(not(feature = "wasm"))]
    fn draw_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x >= 256 || y >= 240 {
            return;
        }
        let index = (x + y * 256) as usize;
        self.buffer[index * 3] = color.r;
        self.buffer[index * 3 + 1] = color.g;
        self.buffer[index * 3 + 2] = color.b;
    }

    #[cfg(feature = "wasm")]
    fn draw_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x >= 256 || y >= 240 {
            return;
        }
        let index = (x + y * 256) as usize;
        self.buffer[index * 4] = color.r;
        self.buffer[index * 4 + 1] = color.g;
        self.buffer[index * 4 + 2] = color.b;
        self.buffer[index * 4 + 3] = 0xFF;
    }

    fn sample_palette_ram(&self, palette: u8, index: u8) -> u8 {
        self.ppu_read(0x3F00 + ((palette << 2) + index) as u16)
    }
}

#[bitfield_struct::bitfield(u16)]
#[derive(PartialEq, Eq)]
struct VramAddress {
    #[bits(5)]
    coarse_x: u16,
    #[bits(5)]
    coarse_y: u16,
    #[bits(1)]
    nametable_x: u16,
    #[bits(1)]
    nametable_y: u16,
    #[bits(3)]
    fine_y: u16,
    #[bits(1)]
    __: u16,
}

#[bitfield_struct::bitfield(u8)]
#[derive(PartialEq, Eq)]
struct PpuControl {
    #[bits(2)]
    nametable: u8,
    #[bits(1)]
    address_increment: u8,
    #[bits(1)]
    sprite_pattern: u8,
    #[bits(1)]
    background_pattern: u8,
    #[bits(1)]
    sprite_size: u8,
    #[bits(1)]
    ppu_master_slave: u8,
    nmi: bool,
}

#[bitfield_struct::bitfield(u8)]
#[derive(PartialEq, Eq)]
struct PpuMask {
    grayscale: bool,
    show_left_background_tiles: bool,
    show_left_sprite_tiles: bool,
    show_background: bool,
    show_sprites: bool,
    emphasize_red: bool,
    emphasize_green: bool,
    emphasize_blue: bool,
}

#[bitfield_struct::bitfield(u8)]
#[derive(PartialEq, Eq)]
struct PpuStatus {
    #[bits(5)]
    open_bus: u8,
    sprite_overflow: bool,
    sprite_zero_hit: bool,
    vblank: bool,
}
