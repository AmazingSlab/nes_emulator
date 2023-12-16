use std::{cell::RefCell, rc::Rc};

use crate::{Cartridge, Controller, Cpu, Ppu};

#[derive(Debug)]
pub struct Bus {
    cpu: Rc<RefCell<Cpu>>,
    ram: [u8; 2048],
    ppu: Rc<RefCell<Ppu>>,
    cartridge: Rc<RefCell<Cartridge>>,
    controller: Controller,
    controller_state: Controller,
    controller_strobe: bool,
}

impl Bus {
    pub fn new(
        cpu: Rc<RefCell<Cpu>>,
        ram: [u8; 2048],
        ppu: Rc<RefCell<Ppu>>,
        cartridge: Rc<RefCell<Cartridge>>,
    ) -> Rc<RefCell<Self>> {
        let bus = Self {
            cpu,
            ram,
            ppu,
            cartridge,
            controller: Controller::default(),
            controller_state: Controller::default(),
            controller_strobe: false,
        };

        Rc::new_cyclic(|rc| {
            bus.cpu.borrow_mut().connect_bus(rc.clone());
            bus.ppu.borrow_mut().connect_bus(rc.clone());
            bus.cartridge.borrow_mut().connect_bus(rc.clone());
            RefCell::new(bus)
        })
    }

    pub fn set_controller_state(&mut self, controller_state: Controller) {
        self.controller = controller_state;
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[addr as usize & 0x07FF],
            0x2000..=0x3FFF => self.ppu.borrow_mut().cpu_read(addr & 0x07),
            0x4016 => {
                if self.controller_strobe {
                    self.controller_state = self.controller;
                }
                let data = self.controller_state.0 & 0x01;
                self.controller_state.0 >>= 1;
                data
            }
            0x4020..=0xFFFF => self.cartridge.borrow().cpu_read(addr),
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram[addr as usize & 0x07FF] = data,
            0x2000..=0x3FFF => self.ppu.borrow_mut().cpu_write(addr & 0x07, data),
            0x4016 => {
                self.controller_strobe = (data & 0x01) != 0;
                self.controller_state = self.controller;
            }
            0x4020..=0xFFFF => self.cartridge.borrow_mut().cpu_write(addr, data),
            _ => (),
        }
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cartridge.borrow().ppu_read(addr),
            _ => 0,
        }
    }

    pub fn ppu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.cartridge.borrow_mut().ppu_write(addr, data),
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

    pub fn reset(cpu: Rc<RefCell<Cpu>>, ppu: Rc<RefCell<Ppu>>) {
        cpu.borrow_mut().reset();
        ppu.borrow_mut().reset();
    }
}
