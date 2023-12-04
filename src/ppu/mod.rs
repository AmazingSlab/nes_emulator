use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use rand::Rng;

use crate::Bus;

#[derive(Debug)]
pub struct Ppu {
    bus: Weak<RefCell<Bus>>,
    pub buffer: [u8; 256 * 240 * 3],
    coords: (u16, u16),
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
        let (x, y) = self.coords;

        let color = if rand::thread_rng().gen() {
            Color::new(255, 255, 255)
        } else {
            Color::new(0, 0, 0)
        };
        self.draw_pixel(x, y, color);
        self.coords.0 += 1;
        if x > 341 {
            self.coords.0 = 0;
            self.coords.1 += 1;
        }
        if y > 256 {
            self.coords.1 = 0;
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
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            bus: Weak::new(),
            buffer: [0; 256 * 240 * 3],
            coords: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}
