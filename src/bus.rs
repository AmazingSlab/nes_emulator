use std::{cell::RefCell, rc::Rc};

use crate::{Cartridge, Cpu};

#[derive(Debug)]
pub struct Bus {
    cpu: Rc<RefCell<Cpu>>,
    ram: [u8; 2048],
    cartridge: Cartridge,
}

impl Bus {
    pub fn new(cpu: Rc<RefCell<Cpu>>, ram: [u8; 2048], cartridge: Cartridge) -> Rc<RefCell<Self>> {
        let mut bus = Self {
            cpu,
            ram,
            cartridge,
        };

        Rc::new_cyclic(|rc| {
            bus.cpu.borrow_mut().connect_bus(rc.clone());
            bus.cartridge.connect_bus(rc.clone());
            RefCell::new(bus)
        })
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[addr as usize & 0x07FF],
            0x4020..=0xFFFF => self.cartridge.read(addr),
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram[addr as usize & 0x07FF] = data,
            0x4020..=0xFFFF => self.cartridge.write(addr, data),
            _ => (),
        }
    }
}
