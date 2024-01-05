mod mapper_0;

pub use mapper_0::Mapper0;

pub trait Mapper: std::fmt::Debug {
    fn cpu_read(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, data: u8);
    fn ppu_read(&self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, data: u8);
    fn mirroring(&self) -> Mirroring;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}
