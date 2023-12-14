use std::{fmt::Display, ops::Deref};

use num_enum::{IntoPrimitive, TryFromPrimitive, TryFromPrimitiveError};

use super::{utils, ArrayKey, Color};

#[repr(u8)]
#[derive(IntoPrimitive, TryFromPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    None = 0,
    Pawn = 1,
    Knight = 2,
    Bishop = 3,
    Rook = 4,
    Queen = 5,
    King = 6,
}

impl Piece {
    pub const ALL: &'static [Self] = &[
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];

    pub const ALL_INCLUDING_NONE: &'static [Self] = &[
        Self::None,
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];
}

impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<char>::into(*self))
    }
}

impl Into<char> for Piece {
    fn into(self) -> char {
        const PIECES: &'static [u8] = " PNBRQK".as_bytes();
        PIECES[self as usize] as char
    }
}

impl TryFrom<PieceIndex> for Piece {
    type Error = TryFromPrimitiveError<Self>;
    fn try_from(value: PieceIndex) -> Result<Self, Self::Error> {
        Piece::try_from_primitive(value.0 & 0b111)
    }
}

impl Into<usize> for Piece {
    fn into(self) -> usize {
        self as usize
    }
}

impl From<Piece> for utils::Index {
    fn from(value: Piece) -> Self {
        utils::Index(value as usize)
    }
}

impl ArrayKey for Piece {
    const COUNT: usize = 7;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceIndex(pub u8);

impl PieceIndex {
    pub const NONE: Self = Self(0);

    pub fn new(color: Color, piece: Piece) -> Self {
        PieceIndex(u8::from(color) << 3 | u8::from(piece))
    }

    pub fn color(self) -> Color {
        Color::try_from(self).unwrap()
    }

    pub fn piece(self) -> Piece {
        Piece::try_from(self).unwrap()
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn some(self) -> bool {
        self != Self::NONE
    }
}

impl Deref for PieceIndex {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for PieceIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c: char = self.piece().into();
        if self.color() == Color::White {
            write!(f, "{}", c.to_ascii_uppercase())
        } else {
            write!(f, "{}", c.to_ascii_lowercase())
        }
    }
}

impl Into<usize> for PieceIndex {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl From<PieceIndex> for utils::Index {
    fn from(value: PieceIndex) -> Self {
        utils::Index(value.index())
    }
}

impl From<(Color, Piece)> for PieceIndex {
    fn from(value: (Color, Piece)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl ArrayKey for PieceIndex {
    const COUNT: usize = 16;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_bit_twidling() {
        let index = PieceIndex::new(Color::White, Piece::Queen);
        let (color, piece) = (index.color(), index.piece());
        assert_eq!(color, Color::White);
        assert_eq!(piece, Piece::Queen);
        assert_eq!(index, PieceIndex::new(color, piece));
    }

    #[test]
    fn test_piece_index_sizing() {
        for color in Color::ALL.iter() {
            for piece in Piece::ALL_INCLUDING_NONE.iter() {
                let index = PieceIndex::new(*color, *piece);
                assert!(index.index() <= 14, "index={:?}", index.index());
            }
        }
    }
}
