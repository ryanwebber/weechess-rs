use std::fmt::Display;

use num_enum::{IntoPrimitive, TryFromPrimitive, TryFromPrimitiveError};

use super::{utils, utils::ArrayKey, Offset, PieceIndex};

#[repr(u8)]
#[derive(IntoPrimitive, TryFromPrimitive, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const ALL: &'static [Self] = &[Self::White, Self::Black];

    pub fn opposing_color(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    pub fn forward(self) -> Offset {
        match self {
            Self::White => Offset::NORTH,
            Self::Black => Offset::SOUTH,
        }
    }

    pub fn backward(self) -> Offset {
        match self {
            Self::White => Offset::SOUTH,
            Self::Black => Offset::NORTH,
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const COLORS: &'static [u8] = "wb".as_bytes();
        if *self as u8 > 1 {
            write!(f, "?")
        } else {
            write!(f, "{}", COLORS[*self as usize] as char)
        }
    }
}

impl TryFrom<PieceIndex> for Color {
    type Error = TryFromPrimitiveError<Self>;
    fn try_from(value: PieceIndex) -> Result<Self, Self::Error> {
        Color::try_from_primitive(value.0 >> 3)
    }
}

impl From<Color> for utils::Index {
    fn from(value: Color) -> Self {
        utils::Index(value as usize)
    }
}

impl ArrayKey for Color {
    const COUNT: usize = 2;
}
