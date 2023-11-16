use crate::Memory;

#[derive(Debug, Default)]
pub struct Bus {
    memory: Memory,
}

impl Bus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_memory(memory: Memory) -> Self {
        Self { memory }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}
