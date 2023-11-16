use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct Memory {
    // Box array to allocate on the heap.
    memory: Box<[u8; 64 * 1024]>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            memory: Box::new([0; 64 * 1024]),
        }
    }

    pub fn with_data(data: [u8; 64 * 1024]) -> Self {
        Self {
            memory: Box::new(data),
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Memory {
    type Target = [u8; 64 * 1024];

    fn deref(&self) -> &Self::Target {
        &self.memory
    }
}

impl DerefMut for Memory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.memory
    }
}
