// TODO: Remove
#![allow(unused)]

pub struct Savestate<'a> {
    pub(crate) header: Header,
    pub(crate) cpu_state: CpuState,
    pub(crate) ppu_state: PpuState,
    pub(crate) mapper_state: MapperState<'a>,
}

impl<'a> Savestate<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, String> {
        if bytes.len() < 3 || &bytes[0..3] != b"FCS" {
            return Err("not a savestate".into());
        }
        if bytes.len() < 16 {
            return Err("header ended unexpectedly".into());
        }

        let (header, rest) = bytes.split_at(16);

        let header = Header::new(header)?;

        if header.compressed_size.is_some() {
            return Err("compressed savestates not supported".into());
        }

        if rest.len() != header.file_size as usize {
            return Err("file size doesn't match header".into());
        }

        if rest.len() < 5 {
            return Err("section header ended unexpectedly".into());
        }

        let mut cpu_state = None;
        let mut ppu_state = None;
        let mut mapper_state = None;

        let mut bytes = rest;

        while !bytes.is_empty() {
            let (section_header, rest) = bytes.split_at(5);
            let section_kind = SectionChunkKind::new(section_header[0]);
            let section_size =
                u32::from_le_bytes(section_header[1..5].try_into().unwrap()) as usize;

            let (section, rest) = rest.split_at(section_size);
            bytes = rest;

            match section_kind {
                SectionChunkKind::Cpu => cpu_state = Some(CpuState::new(section)?),
                SectionChunkKind::Ppu => ppu_state = Some(PpuState::new(section)?),
                SectionChunkKind::Extra => mapper_state = Some(MapperState::new(section)?),
                _ => (), // TODO
            };
        }

        Ok(Self {
            header,
            cpu_state: cpu_state.unwrap(),
            ppu_state: ppu_state.unwrap(),
            mapper_state: mapper_state.unwrap(),
        })
    }
}

#[derive(Debug)]
pub struct Header {
    old_version: u8,
    file_size: u32,
    version: u32,
    compressed_size: Option<u32>,
}

impl Header {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        let old_version = bytes[3];
        let file_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let compressed_size = u32::from_le_bytes(bytes[12..16].try_into().unwrap());

        let compressed_size = match compressed_size {
            0x00000000 | 0xFFFFFFFF => None,
            x => Some(x),
        };

        Ok(Header {
            old_version,
            file_size,
            version,
            compressed_size,
        })
    }
}

#[derive(Debug)]
enum SectionChunkKind {
    Cpu,
    Cpuc,
    Ppu,
    Ctlr,
    Snd,
    Extra,
    Unknown,
}

impl SectionChunkKind {
    pub fn new(byte: u8) -> Self {
        match byte {
            1 => Self::Cpu,
            2 => Self::Cpuc,
            3 => Self::Ppu,
            4 => Self::Ctlr,
            5 => Self::Snd,
            16 => Self::Extra,
            _ => Self::Unknown,
        }
    }
}

pub struct CpuState {
    pub(crate) accumulator: u8,
    pub(crate) x_register: u8,
    pub(crate) y_register: u8,
    pub(crate) program_counter: u16,
    pub(crate) stack_pointer: u8,
    pub(crate) status: u8,
    pub(crate) data_bus: u8,
    pub(crate) ram: Box<[u8; 2048]>,
}

impl CpuState {
    fn new(bytes: &[u8]) -> Result<Self, String> {
        let mut accumulator = 0;
        let mut x_register = 0;
        let mut y_register = 0;
        let mut program_counter = 0;
        let mut stack_pointer = 0;
        let mut status = 0;
        let mut data_bus = 0;
        let mut ram = None;

        let subchunk = Subchunk::new(bytes)?;
        for (description, section) in subchunk {
            match description {
                "PC" => program_counter = deserialize(section)?,
                "A" => accumulator = deserialize(section)?,
                "P" => status = deserialize(section)?,
                "X" => x_register = deserialize(section)?,
                "Y" => y_register = deserialize(section)?,
                "S" => stack_pointer = deserialize(section)?,
                "DB" => data_bus = deserialize(section)?,
                "RAM" => ram = Some(deserialize(section)?),
                _ => println!("warn: unrecognized section `{description}`"),
            }
        }

        Ok(Self {
            accumulator,
            x_register,
            y_register,
            program_counter,
            stack_pointer,
            status,
            data_bus,
            ram: ram.unwrap_or_else(crate::new_boxed_array),
        })
    }
}

pub struct PpuState {
    pub(crate) nametables: Box<[u8; 2048]>,
    pub(crate) palette_ram: Box<[u8; 32]>,
    pub(crate) oam: Box<[u8; 256]>,

    pub(crate) control: u8,
    pub(crate) mask: u8,
    pub(crate) status: u8,
    pub(crate) oam_addr: u8,

    pub(crate) tile_x_offset: u8,
    pub(crate) addr_latch: u8,
    pub(crate) vram_addr: u16,
    pub(crate) temp_vram_addr: u16,
    pub(crate) data_buffer: u8,
    pub(crate) general_latch: u8,
}

impl PpuState {
    fn new(bytes: &[u8]) -> Result<Self, String> {
        let mut nametables = None;
        let mut palette_ram = None;
        let mut oam = None;

        let mut control = 0;
        let mut mask = 0;
        let mut status = 0;
        let mut oam_addr = 0;

        let mut tile_x_offset = 0;
        let mut addr_latch = 0;
        let mut vram_addr = 0;
        let mut temp_vram_addr = 0;
        let mut data_buffer = 0;
        let mut general_latch = 0;

        let subchunk = Subchunk::new(bytes)?;
        for (description, section) in subchunk {
            match description {
                "NTAR" => nametables = Some(deserialize(section)?),
                "PRAM" => palette_ram = Some(deserialize(section)?),
                "SPRA" => oam = Some(deserialize(section)?),
                "PPUR" => [control, mask, status, oam_addr] = deserialize(section)?,
                "XOFF" => tile_x_offset = deserialize(section)?,
                "VTGL" => addr_latch = deserialize(section)?,
                "RADD" => vram_addr = deserialize(section)?,
                "TADD" => temp_vram_addr = deserialize(section)?,
                "VBUF" => data_buffer = deserialize(section)?,
                "PGEN" => general_latch = deserialize(section)?,
                _ => println!("warn: unrecognized section `{description}`"),
            }
        }

        Ok(Self {
            nametables: nametables.unwrap_or_else(crate::new_boxed_array),
            palette_ram: palette_ram.unwrap_or_else(crate::new_boxed_array),
            oam: oam.unwrap_or_else(crate::new_boxed_array),
            control,
            mask,
            status,
            oam_addr,
            tile_x_offset,
            addr_latch,
            vram_addr,
            temp_vram_addr,
            data_buffer,
            general_latch,
        })
    }
}

pub struct MapperState<'a> {
    subchunk: Subchunk<'a>,
}

impl<'a> MapperState<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, String> {
        Ok(Self {
            subchunk: Subchunk::new(bytes)?,
        })
    }
}

impl<'a> IntoIterator for MapperState<'a> {
    type Item = <Subchunk<'a> as IntoIterator>::Item;
    type IntoIter = <Subchunk<'a> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.subchunk.into_iter()
    }
}

pub struct Subchunk<'a> {
    sections: Vec<(&'a str, &'a [u8])>,
}

impl<'a> Subchunk<'a> {
    pub fn new(mut bytes: &'a [u8]) -> Result<Self, String> {
        let mut sections = Vec::new();

        while !bytes.is_empty() {
            if bytes.len() < 8 {
                return Err("chunk header ended unexpectedly".into());
            }

            let (header, rest) = bytes.split_at(8);
            let size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
            if rest.len() < size {
                return Err("chunk length doesn't match header".into());
            }

            let (section, rest) = rest.split_at(size);
            bytes = rest;

            let description = std::str::from_utf8(&header[0..4])
                .map_err(|_| "invalid chunk description")?
                .trim_end_matches('\0');
            sections.push((description, section));
        }

        Ok(Self { sections })
    }
}

impl<'a> IntoIterator for Subchunk<'a> {
    type Item = (&'a str, &'a [u8]);
    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    /// Iterates over tuples of a chunk's description and data.
    fn into_iter(self) -> Self::IntoIter {
        self.sections.into_iter()
    }
}

pub fn deserialize<T: FromBytes>(bytes: &[u8]) -> Result<T, String> {
    T::from_bytes(bytes).ok_or_else(|| "invalid section size".into())
}

pub trait FromBytes: Sized {
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

impl FromBytes for u8 {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(u8::from_le_bytes(bytes.try_into().ok()?))
    }
}

impl FromBytes for u16 {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(u16::from_le_bytes(bytes.try_into().ok()?))
    }
}

impl FromBytes for u32 {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }
}

impl<const N: usize> FromBytes for Box<[u8; N]> {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Box::new(bytes.try_into().ok()?))
    }
}

impl<const N: usize> FromBytes for [u8; N] {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bytes.try_into().ok()
    }
}

impl FromBytes for Vec<u8> {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes.into())
    }
}