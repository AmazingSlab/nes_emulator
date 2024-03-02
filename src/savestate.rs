// TODO: Remove
#![allow(unused)]

use std::{borrow::Cow, io::Read};

use flate2::read::ZlibDecoder;

pub struct Savestate<'a> {
    pub(crate) header: Header,
    pub(crate) cpu_state: CpuState,
    pub(crate) ppu_state: PpuState,
    pub(crate) apu_state: ApuState,
    pub(crate) mapper_state: MapperState<'a>,
}

impl<'a> Savestate<'a> {
    /// Parses an uncompressed FCEUX FCS savestate file.
    ///
    /// To parse a compressed savestate, see [Savestate::decompress].
    ///
    /// # Errors
    ///
    /// Returns an error if the file is malformed or compressed.
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
            return Err("savestate is compressed".into());
        }

        if rest.len() != header.file_size as usize {
            return Err("file size doesn't match header".into());
        }

        if rest.len() < 5 {
            return Err("section header ended unexpectedly".into());
        }

        let mut cpu_state = None;
        let mut ppu_state = None;
        let mut apu_state = None;
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
                SectionChunkKind::Snd => apu_state = Some(ApuState::new(section)?),
                SectionChunkKind::Extra => mapper_state = Some(MapperState::new(section)?),
                _ => (), // TODO
            };
        }

        Ok(Self {
            header,
            cpu_state: cpu_state.unwrap(),
            ppu_state: ppu_state.unwrap(),
            apu_state: apu_state.unwrap(),
            mapper_state: mapper_state.unwrap(),
        })
    }

    /// Decompresses a compressed FCEUX FCS savestate file.
    ///
    /// Use in conjunction with [Savestate::new] to parse the returned data.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is malformed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_emulator::Savestate;
    ///
    /// # fn main() -> Result<(), String> {
    /// # let bytes = Vec::new();
    /// let decompressed = Savestate::decompress(&bytes)?;
    /// let savestate = Savestate::new(&decompressed)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn decompress(bytes: &'a [u8]) -> Result<Cow<'a, [u8]>, String> {
        if bytes.len() < 3 || &bytes[0..3] != b"FCS" {
            return Err("not a savestate".into());
        }
        if bytes.len() < 16 {
            return Err("header ended unexpectedly".into());
        }

        let (header_bytes, rest) = bytes.split_at(16);

        let header = Header::new(header_bytes)?;

        match header.compressed_size {
            Some(compressed_size) => {
                if rest.len() != compressed_size as usize {
                    return Err("compressed size doesn't match header".into());
                }

                let mut decoder = ZlibDecoder::new(rest);

                let expected_output_size = header_bytes.len() + header.file_size as usize;
                let mut output = vec![0u8; expected_output_size];

                // Copy header into the output buffer while indicating that the data is
                // uncompressed.
                output[0..12].copy_from_slice(&header_bytes[0..12]);
                output[12..16].fill(0xFF);

                // Decompress data into the main body of the output buffer.
                decoder.read(&mut output[16..]);

                Ok(Cow::Owned(output))
            }
            None => Ok(Cow::Borrowed(bytes)),
        }
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

pub struct ApuState {
    /// All values from 0x4000-0x400F for channels 1-4, unused bytes included.
    pub(crate) channel_data: [u8; 16],
    pub(crate) channel_enables: u8,
    pub(crate) frame_mode: u8,
    pub(crate) noise_shift_register: u16,
    pub(crate) triangle_linear_counter_reload_flag: bool,
    pub(crate) triangle_linear_counter: u8,

    pub(crate) pulse_1_envelope_divider_reload: u8,
    pub(crate) pulse_2_envelope_divider_reload: u8,
    pub(crate) noise_envelope_divider_reload: u8,

    pub(crate) pulse_1_envelope_mode: u8,
    pub(crate) pulse_2_envelope_mode: u8,
    pub(crate) noise_envelope_mode: u8,

    pub(crate) pulse_1_envelope_divider: u8,
    pub(crate) pulse_2_envelope_divider: u8,
    pub(crate) noise_envelope_divider: u8,

    pub(crate) pulse_1_envelope_decay_level: u8,
    pub(crate) pulse_2_envelope_decay_level: u8,
    pub(crate) noise_envelope_decay_level: u8,

    pub(crate) pulse_1_length_counter: u8,
    pub(crate) pulse_2_length_counter: u8,
    pub(crate) triangle_length_counter: u8,
    pub(crate) noise_length_counter: u8,

    pub(crate) is_pulse_1_sweep_enabled: bool,
    pub(crate) is_pulse_2_sweep_enabled: bool,

    pub(crate) pulse_1_sweep_target_period: u16,
    pub(crate) pulse_2_sweep_target_period: u16,

    pub(crate) pulse_1_sweep_divider: u8,
    pub(crate) pulse_2_sweep_divider: u8,
}

impl ApuState {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        let mut channel_data = None;
        let mut channel_enables = 0;
        let mut frame_mode = 0;
        let mut noise_shift_register = 1;
        let mut triangle_linear_counter_reload_flag = false;
        let mut triangle_linear_counter = 0;

        let mut pulse_1_envelope_divider_reload = 0;
        let mut pulse_2_envelope_divider_reload = 0;
        let mut noise_envelope_divider_reload = 0;

        let mut pulse_1_envelope_mode = 0;
        let mut pulse_2_envelope_mode = 0;
        let mut noise_envelope_mode = 0;

        let mut pulse_1_envelope_divider = 0;
        let mut pulse_2_envelope_divider = 0;
        let mut noise_envelope_divider = 0;

        let mut pulse_1_envelope_decay_level = 0;
        let mut pulse_2_envelope_decay_level = 0;
        let mut noise_envelope_decay_level = 0;

        let mut pulse_1_length_counter = 0;
        let mut pulse_2_length_counter = 0;
        let mut triangle_length_counter = 0;
        let mut noise_length_counter = 0;

        let mut is_pulse_1_sweep_enabled = false;
        let mut is_pulse_2_sweep_enabled = false;

        let mut pulse_1_sweep_target_period = 0;
        let mut pulse_2_sweep_target_period = 0;

        let mut pulse_1_sweep_divider = 0;
        let mut pulse_2_sweep_divider = 0;

        let subchunk = Subchunk::new(bytes)?;
        for (description, section) in subchunk {
            match description {
                "FHCN" | "FCNT" => {} // Unsure what these counters are supposed to mean.
                "PSG" => channel_data = Some(deserialize(section)?),
                "ENCH" => channel_enables = deserialize(section)?,
                "IQFM" => frame_mode = deserialize(section)?,
                "NREG" => noise_shift_register = deserialize(section)?,
                "TRIM" => triangle_linear_counter_reload_flag = deserialize(section)?,
                "TRIC" => triangle_linear_counter = deserialize(section)?,

                "E0SP" => pulse_1_envelope_divider_reload = deserialize(section)?,
                "E1SP" => pulse_2_envelope_divider_reload = deserialize(section)?,
                "E2SP" => noise_envelope_divider_reload = deserialize(section)?,

                "E0MO" => pulse_1_envelope_mode = deserialize(section)?,
                "E1MO" => pulse_2_envelope_mode = deserialize(section)?,
                "E2MO" => noise_envelope_mode = deserialize(section)?,

                "E0D1" => pulse_1_envelope_divider = deserialize(section)?,
                "E1D1" => pulse_2_envelope_divider = deserialize(section)?,
                "E2D1" => noise_envelope_divider = deserialize(section)?,

                "E0DV" => pulse_1_envelope_decay_level = deserialize(section)?,
                "E1DV" => pulse_2_envelope_decay_level = deserialize(section)?,
                "E2DV" => noise_envelope_decay_level = deserialize(section)?,

                // FCEUX treats these as u8 but stores them as i32 for some reason.
                "LEN0" => pulse_1_length_counter = deserialize::<u32>(section)? as u8,
                "LEN1" => pulse_2_length_counter = deserialize::<u32>(section)? as u8,
                "LEN2" => triangle_length_counter = deserialize::<u32>(section)? as u8,
                "LEN3" => noise_length_counter = deserialize::<u32>(section)? as u8,

                "SWEE" => {
                    [is_pulse_1_sweep_enabled, is_pulse_2_sweep_enabled] = deserialize(section)?
                }

                // FCEUX treats these as u16 but stores them as i32 for some reason.
                "CRF1" => pulse_1_sweep_target_period = deserialize::<u32>(section)? as u16,
                "CRF2" => pulse_2_sweep_target_period = deserialize::<u32>(section)? as u16,

                "SWCT" => [pulse_1_sweep_divider, pulse_2_sweep_divider] = deserialize(section)?,
                "SIRQ" | "5ACC" | "5BIT" | "5ADD" | "5SIZ" | "5SHF" | "5HVD" | "5HVS" | "5SZL"
                | "5ADL" | "5FMT" | "RWDA" => {} // TODO: DMC channel.
                _ => println!("warn: unrecognized section `{description}`"),
            }
        }

        Ok(Self {
            channel_data: channel_data.unwrap_or_default(),
            channel_enables,
            frame_mode,
            noise_shift_register,
            triangle_linear_counter_reload_flag,
            triangle_linear_counter,

            pulse_1_envelope_divider_reload,
            pulse_2_envelope_divider_reload,
            noise_envelope_divider_reload,

            pulse_1_envelope_mode,
            pulse_2_envelope_mode,
            noise_envelope_mode,

            pulse_1_envelope_divider,
            pulse_2_envelope_divider,
            noise_envelope_divider,

            pulse_1_envelope_decay_level,
            pulse_2_envelope_decay_level,
            noise_envelope_decay_level,

            pulse_1_length_counter,
            pulse_2_length_counter,
            triangle_length_counter,
            noise_length_counter,

            is_pulse_1_sweep_enabled,
            is_pulse_2_sweep_enabled,

            pulse_1_sweep_target_period,
            pulse_2_sweep_target_period,

            pulse_1_sweep_divider,
            pulse_2_sweep_divider,
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

impl FromBytes for bool {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(u8::from_bytes(bytes)? != 0)
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

impl<const N: usize> FromBytes for [bool; N] {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bytes
            .iter()
            .copied()
            .map(|b| b != 0)
            .collect::<Vec<_>>()
            .try_into()
            .ok()
    }
}

impl FromBytes for Vec<u8> {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes.into())
    }
}
