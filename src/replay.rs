use std::{iter::Peekable, str::FromStr};

use crate::Controller;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Replay<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    version: u8,
    emu_version: u32,
    rerecord_count: Option<u32>,
    pal_flag: Option<bool>,
    new_ppu: Option<bool>,
    fds: Option<bool>,
    fourscore: bool,
    microphone: Option<bool>,
    port_0: InputDevice,
    port_1: InputDevice,
    port_2: PortDevice,
    binary: Option<bool>,
    length: Option<u32>,
    rom_filename: String,
    comment: Option<String>,
    guid: String,
    rom_checksum: String,
    savestate: Option<String>,
    iter: Peekable<I>,
}

impl<'a, I> Replay<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    pub fn new(input: I) -> Result<Self, String> {
        let mut iter = input.peekable();
        let mut builder = ReplayBuilder::new();

        fn parse<T: FromStr>(key: &str, value: &str) -> Result<T, String> {
            value
                .parse()
                .map_err(|_| format!("`{value}` is not a valid value for key `{key}`"))
        }

        while let Some(&next) = iter.peek() {
            if next.starts_with('|') {
                // Beginning of input log; stop parsing header.
                break;
            }

            if let Some(line) = iter.next() {
                let Some((key, value)) = line.split_once(' ') else {
                    return Err(format!("`{line}` is not a valid entry"));
                };

                match key {
                    "version" => builder.set_version(parse(key, value)?),
                    "emuVersion" => builder.set_emu_version(parse(key, value)?),
                    "rerecordCount" => builder.set_rerecord_count(parse(key, value)?),
                    "palFlag" => builder.set_pal_flag(parse::<u8>(key, value)? != 0),
                    "NewPPU" => builder.set_new_ppu(parse::<u8>(key, value)? != 0),
                    "FDS" => builder.set_fds(parse::<u8>(key, value)? != 0),
                    "fourscore" => builder.set_fourscore(parse::<u8>(key, value)? != 0),
                    "microphone" => builder.set_microphone(parse::<u8>(key, value)? != 0),
                    "port0" => builder.set_port_0(parse::<u8>(key, value)?.try_into()?),
                    "port1" => builder.set_port_1(parse::<u8>(key, value)?.try_into()?),
                    "port2" => builder.set_port_2(parse::<u8>(key, value)?.try_into()?),
                    "binary" => builder.set_binary(parse(key, value)?),
                    "length" => builder.set_length(parse(key, value)?),
                    "romFilename" => builder.set_rom_filename(value.to_string()),
                    "comment" => builder.set_comment(value.to_string()),
                    // Multiple subtitle entries with different timings are possible and will
                    // require special handling. Do nothing for now.
                    "subtitle" => &mut builder,
                    "guid" => builder.set_guid(value.to_string()),
                    "romChecksum" => builder.set_rom_checksum(value.to_string()),
                    "savestate" => builder.set_savestate(value.to_string()),
                    _ => return Err(format!("unrecognized key `{key}`")),
                };
            }
        }

        let replay = builder.build(iter)?;

        if replay.version != 3 {
            return Err(format!("invalid version number `{}`", replay.version));
        }
        if replay.pal_flag.unwrap_or_default() {
            return Err("pal not supported".into());
        }
        if replay.fds.unwrap_or_default() {
            return Err("fds not supported".into());
        }
        if replay.fourscore {
            return Err("fourscore not supported".into());
        }
        if replay.microphone.unwrap_or_default() {
            return Err("microphone not supported".into());
        }
        if replay.binary.unwrap_or_default() {
            return Err("binary input log not supported".into());
        }
        if replay.savestate.is_some() {
            return Err("savestates not supported".into());
        }

        Ok(replay)
    }
}

impl<'a, I> Iterator for Replay<'a, I>
where
    I: Iterator<Item = &'a str>,
{
    type Item = (InputCommand, Controller, Controller);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|line| {
            let (_, line) = line.split_once('|')?;
            let (command, line) = line.split_once('|')?;
            let (controller_1, line) = line.split_once('|')?;
            let (controller_2, line) = line.split_once('|')?;
            let (port_2, _) = line.split_once('|')?;

            // Port 2 must be empty.
            if !port_2.is_empty() {
                return None;
            }

            let command: InputCommand = command.parse::<u8>().ok()?.into();
            let controller_1 = parse_controller(controller_1);
            let controller_2 = parse_controller(controller_2);

            Some((command, controller_1, controller_2))
        })?
    }
}

fn parse_controller(controller: &str) -> Controller {
    if controller.len() != 8 {
        return Controller::default();
    }
    let iter = controller.chars().enumerate();
    let mut controller = Controller::new();
    for (i, char) in iter {
        let is_pressed = char != ' ' && char != '.';
        match i {
            0 => controller.set_right(is_pressed),
            1 => controller.set_left(is_pressed),
            2 => controller.set_down(is_pressed),
            3 => controller.set_up(is_pressed),
            4 => controller.set_start(is_pressed),
            5 => controller.set_select(is_pressed),
            6 => controller.set_b(is_pressed),
            7 => controller.set_a(is_pressed),
            _ => unreachable!(),
        }
    }
    controller
}

#[derive(Default)]
struct ReplayBuilder {
    version: Option<u8>,
    emu_version: Option<u32>,
    rerecord_count: Option<u32>,
    pal_flag: Option<bool>,
    new_ppu: Option<bool>,
    fds: Option<bool>,
    fourscore: Option<bool>,
    microphone: Option<bool>,
    port_0: Option<InputDevice>,
    port_1: Option<InputDevice>,
    port_2: Option<PortDevice>,
    binary: Option<bool>,
    length: Option<u32>,
    rom_filename: Option<String>,
    comment: Option<String>,
    guid: Option<String>,
    rom_checksum: Option<String>,
    savestate: Option<String>,
}

impl ReplayBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn set_version(&mut self, version: u8) -> &mut Self {
        self.version = Some(version);
        self
    }
    fn set_emu_version(&mut self, emu_version: u32) -> &mut Self {
        self.emu_version = Some(emu_version);
        self
    }
    fn set_rerecord_count(&mut self, rerecord_count: u32) -> &mut Self {
        self.rerecord_count = Some(rerecord_count);
        self
    }
    fn set_pal_flag(&mut self, pal_flag: bool) -> &mut Self {
        self.pal_flag = Some(pal_flag);
        self
    }
    fn set_new_ppu(&mut self, new_ppu: bool) -> &mut Self {
        self.new_ppu = Some(new_ppu);
        self
    }
    fn set_fds(&mut self, fds: bool) -> &mut Self {
        self.fds = Some(fds);
        self
    }
    fn set_fourscore(&mut self, fourscore: bool) -> &mut Self {
        self.fourscore = Some(fourscore);
        self
    }
    fn set_microphone(&mut self, microphone: bool) -> &mut Self {
        self.microphone = Some(microphone);
        self
    }
    fn set_port_0(&mut self, port_0: InputDevice) -> &mut Self {
        self.port_0 = Some(port_0);
        self
    }
    fn set_port_1(&mut self, port_1: InputDevice) -> &mut Self {
        self.port_1 = Some(port_1);
        self
    }
    fn set_port_2(&mut self, port_2: PortDevice) -> &mut Self {
        self.port_2 = Some(port_2);
        self
    }
    fn set_binary(&mut self, binary: bool) -> &mut Self {
        self.binary = Some(binary);
        self
    }
    fn set_length(&mut self, length: u32) -> &mut Self {
        self.length = Some(length);
        self
    }
    fn set_rom_filename(&mut self, rom_filename: String) -> &mut Self {
        self.rom_filename = Some(rom_filename);
        self
    }
    fn set_comment(&mut self, comment: String) -> &mut Self {
        self.comment = Some(comment);
        self
    }
    fn set_guid(&mut self, guid: String) -> &mut Self {
        self.guid = Some(guid);
        self
    }
    fn set_rom_checksum(&mut self, rom_checksum: String) -> &mut Self {
        self.rom_checksum = Some(rom_checksum);
        self
    }
    fn set_savestate(&mut self, savestate: String) -> &mut Self {
        self.savestate = Some(savestate);
        self
    }

    fn build<'a, I>(self, iter: Peekable<I>) -> Result<Replay<'a, I>, String>
    where
        I: Iterator<Item = &'a str>,
    {
        let missing_field = |field: &str| format!("missing required field `{field}`");

        let Some(version) = self.version else {
            return Err(missing_field("version"));
        };
        let Some(emu_version) = self.emu_version else {
            return Err(missing_field("emuVersion"));
        };
        let Some(fourscore) = self.fourscore else {
            return Err(missing_field("fourscore"));
        };
        let Some(port_0) = self.port_0 else {
            return Err(missing_field("port0"));
        };
        let Some(port_1) = self.port_1 else {
            return Err(missing_field("port2"));
        };
        let Some(port_2) = self.port_2 else {
            return Err(missing_field("port2"));
        };
        let Some(rom_filename) = self.rom_filename else {
            return Err(missing_field("romFilename"));
        };
        let Some(guid) = self.guid else {
            return Err(missing_field("guid"));
        };
        let Some(rom_checksum) = self.rom_checksum else {
            return Err(missing_field("romChecksum"));
        };

        Ok(Replay {
            version,
            emu_version,
            rerecord_count: self.rerecord_count,
            pal_flag: self.pal_flag,
            new_ppu: self.new_ppu,
            fds: self.fds,
            fourscore,
            microphone: self.microphone,
            port_0,
            port_1,
            port_2,
            binary: self.binary,
            length: self.length,
            rom_filename,
            comment: self.comment,
            guid,
            rom_checksum,
            savestate: self.savestate,
            iter,
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputDevice {
    #[default]
    None,
    Gamepad,
    Zapper,
}

impl InputDevice {
    pub fn new(id: u8) -> Result<Self, String> {
        let device = match id {
            0 => Self::None,
            1 => Self::Gamepad,
            2 => Self::Zapper,
            _ => return Err(format!("invalid input device: {id}")),
        };

        Ok(device)
    }
}

impl TryFrom<u8> for InputDevice {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PortDevice {
    #[default]
    None,
}

impl PortDevice {
    pub fn new(id: u8) -> Result<Self, String> {
        let device = match id {
            0 => Self::None,
            _ => return Err(format!("invalid port device: {id}")),
        };

        Ok(device)
    }
}

impl TryFrom<u8> for PortDevice {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[bitfield_struct::bitfield(u8)]
pub struct InputCommand {
    #[bits(1)]
    pub soft_reset: bool,
    #[bits(1)]
    pub hard_reset: bool,
    #[bits(1)]
    pub disk_insert: bool,
    #[bits(1)]
    pub disk_select: bool,
    #[bits(1)]
    pub insert_coin: bool,
    #[bits(3)]
    __: u8,
}
