mod mapper_0;
mod mapper_1;
mod mapper_2;
mod mapper_4;

pub use mapper_0::Mapper0;
pub use mapper_1::Mapper1;
pub use mapper_2::Mapper2;
pub use mapper_4::Mapper4;

use crate::savestate::MapperState;

pub trait Mapper {
    fn cpu_read(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, data: u8);
    fn ppu_read(&self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, data: u8);
    fn mirroring(&self) -> Mirroring;
    fn check_irq(&self) -> bool {
        false
    }
    fn count_scanline(&mut self) {}
    fn apply_state(&mut self, state: MapperState);
    fn save_state(&self) -> Vec<u8>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreen,
    SingleScreenUpper,
}
