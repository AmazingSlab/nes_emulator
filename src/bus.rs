use std::{cell::RefCell, rc::Rc};

use crate::{Cartridge, Cpu, Ppu};

#[derive(Debug)]
pub struct Bus {
    cpu: Rc<RefCell<Cpu>>,
    ram: [u8; 2048],
    ppu: Rc<RefCell<Ppu>>,
    cartridge: Cartridge,
}

impl Bus {
    pub fn new(
        cpu: Rc<RefCell<Cpu>>,
        ram: [u8; 2048],
        ppu: Rc<RefCell<Ppu>>,
        cartridge: Cartridge,
    ) -> Rc<RefCell<Self>> {
        let mut bus = Self {
            cpu,
            ram,
            ppu,
            cartridge,
        };

        Rc::new_cyclic(|rc| {
            bus.cpu.borrow_mut().connect_bus(rc.clone());
            bus.ppu.borrow_mut().connect_bus(rc.clone());
            bus.cartridge.connect_bus(rc.clone());
            RefCell::new(bus)
        })
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[addr as usize & 0x07FF],
            0x2000..=0x3FFF => self.ppu.borrow_mut().cpu_read(addr & 0x07),
            0x4020..=0xFFFF => self.cartridge.cpu_read(addr),
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram[addr as usize & 0x07FF] = data,
            0x2000..=0x3FFF => self.ppu.borrow_mut().cpu_write(addr & 0x07, data),
            0x4020..=0xFFFF => self.cartridge.cpu_write(addr, data),
            _ => (),
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cartridge.ppu_read(addr),
            _ => 0,
        }
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.cartridge.ppu_write(addr, data),
            _ => todo!(),
        }
    }

    pub fn clock(cpu: Rc<RefCell<Cpu>>, ppu: Rc<RefCell<Ppu>>) {
        cpu.borrow_mut().clock();
        for _ in 0..3 {
            ppu.borrow_mut().clock();
        }
        if ppu.borrow().emit_nmi {
            cpu.borrow_mut().nmi();
            ppu.borrow_mut().emit_nmi = false;
        }
    }
}
