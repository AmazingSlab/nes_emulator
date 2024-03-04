use std::{ops::Deref, str::FromStr};

pub struct GameGenie {
    codes: Vec<GameGenieCode>,
}

impl GameGenie {
    pub fn new<T: AsRef<str>>(codes: &[T]) -> Result<Self, &'static str> {
        let codes = codes
            .iter()
            .map(|code| GameGenieCode::new(code.as_ref()))
            .collect::<Result<_, _>>()?;

        Ok(Self { codes })
    }

    pub fn codes(&self) -> impl Iterator<Item = GameGenieCode> + '_ {
        self.codes.iter().copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GameGenieCode {
    pub(crate) address: u16,
    pub(crate) value: u8,
    pub(crate) compare: Option<u8>,
}

impl GameGenieCode {
    pub fn new(code: &str) -> Result<Self, &'static str> {
        if !matches!(code.len(), 6 | 8) {
            return Err("invalid code");
        }

        let letters: Vec<_> = code
            .chars()
            .map(GameGenieLetter::try_from)
            .collect::<Result<_, _>>()?;

        let mut address = 0x8000;
        let mut value = 0x00;

        address |= (*letters[3] as u16 & 0b0111) << 12;
        address |= (*letters[4] as u16 & 0b1000) << 8;
        address |= (*letters[5] as u16 & 0b0111) << 8;
        address |= (*letters[1] as u16 & 0b1000) << 4;
        address |= (*letters[2] as u16 & 0b0111) << 4;
        address |= *letters[3] as u16 & 0b1000;
        address |= *letters[4] as u16 & 0b0111;

        value |= (*letters[0] & 0b1000) << 4;
        value |= (*letters[1] & 0b0111) << 4;
        value |= *letters[0] & 0b0111;
        value |= if code.len() == 8 {
            *letters[7]
        } else {
            *letters[5]
        } & 0b1000;

        let compare = if code.len() == 8 {
            let mut compare = 0x00;

            compare |= (*letters[6] & 0b1000) << 4;
            compare |= (*letters[7] & 0b0111) << 4;
            compare |= *letters[5] & 0b1000;
            compare |= *letters[6] & 0b0111;

            Some(compare)
        } else {
            None
        };

        Ok(Self {
            address,
            value,
            compare,
        })
    }
}

impl FromStr for GameGenieCode {
    type Err = &'static str;
    fn from_str(code: &str) -> Result<Self, &'static str> {
        Self::new(code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameGenieLetter(u8);

impl TryFrom<char> for GameGenieLetter {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, &'static str> {
        let mapping = match value {
            'A' => 0x0,
            'P' => 0x1,
            'Z' => 0x2,
            'L' => 0x3,
            'G' => 0x4,
            'I' => 0x5,
            'T' => 0x6,
            'Y' => 0x7,
            'E' => 0x8,
            'O' => 0x9,
            'X' => 0xA,
            'U' => 0xB,
            'K' => 0xC,
            'S' => 0xD,
            'V' => 0xE,
            'N' => 0xF,
            _ => return Err("invalid letter"),
        };
        Ok(Self(mapping))
    }
}

impl Deref for GameGenieLetter {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
