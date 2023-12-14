use std::{
    cell::OnceCell,
    fmt::Display,
    ops::{BitAndAssign, BitOrAssign, Not},
};

use super::{ArrayKey, ArrayMap, Color, Index, Piece, PieceIndex};

type BitSet = bitvec::BitArr!(for 64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct File(u8);

impl File {
    pub const A: Self = Self(0);
    pub const B: Self = Self(1);
    pub const C: Self = Self(2);
    pub const D: Self = Self(3);
    pub const E: Self = Self(4);
    pub const F: Self = Self(5);
    pub const G: Self = Self(6);
    pub const H: Self = Self(7);

    pub const ALL: &'static [Self] = &[
        Self::A,
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::F,
        Self::G,
        Self::H,
    ];

    pub fn from_char(c: char) -> Option<Self> {
        if c < 'a' || c > 'h' {
            None
        } else {
            Some(Self(c as u8 - 'a' as u8))
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        if index > 7 {
            None
        } else {
            Some(Self(index as u8))
        }
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const FILES: &'static [u8] = "abcdefgh".as_bytes();
        if self.0 > 7 {
            write!(f, "?")
        } else {
            write!(f, "{}", FILES[self.0 as usize] as char)
        }
    }
}

impl Into<usize> for File {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl From<File> for Index {
    fn from(value: File) -> Self {
        Index(value.0 as usize)
    }
}

impl ArrayKey for File {
    const COUNT: usize = 8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rank(u8);

impl Rank {
    pub const ONE: Self = Self(0);
    pub const TWO: Self = Self(1);
    pub const THREE: Self = Self(2);
    pub const FOUR: Self = Self(3);
    pub const FIVE: Self = Self(4);
    pub const SIX: Self = Self(5);
    pub const SEVEN: Self = Self(6);
    pub const EIGHT: Self = Self(7);

    pub const ALL: &'static [Self] = &[
        Self::ONE,
        Self::TWO,
        Self::THREE,
        Self::FOUR,
        Self::FIVE,
        Self::SIX,
        Self::SEVEN,
        Self::EIGHT,
    ];

    pub fn from_char(c: char) -> Option<Self> {
        if c < '1' || c > '8' {
            None
        } else {
            Some(Self(c as u8 - '1' as u8))
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        if index > 7 {
            None
        } else {
            Some(Self(index as u8))
        }
    }

    pub fn opposing_rank(self) -> Self {
        debug_assert!(self.0 < 8);
        Self(7 - self.0)
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const RANKS: &'static [u8] = "12345678".as_bytes();
        if self.0 > 7 {
            write!(f, "?")
        } else {
            write!(f, "{}", RANKS[self.0 as usize] as char)
        }
    }
}

impl Into<usize> for Rank {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl From<Rank> for Index {
    fn from(value: Rank) -> Self {
        Index(value.0 as usize)
    }
}

impl ArrayKey for Rank {
    const COUNT: usize = 8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square(u8);

impl Square {
    pub const A1: Self = Self(0);
    pub const B1: Self = Self(1);
    pub const C1: Self = Self(2);
    pub const D1: Self = Self(3);
    pub const E1: Self = Self(4);
    pub const F1: Self = Self(5);
    pub const G1: Self = Self(6);
    pub const H1: Self = Self(7);
    pub const A2: Self = Self(8);
    pub const B2: Self = Self(9);
    pub const C2: Self = Self(10);
    pub const D2: Self = Self(11);
    pub const E2: Self = Self(12);
    pub const F2: Self = Self(13);
    pub const G2: Self = Self(14);
    pub const H2: Self = Self(15);
    pub const A3: Self = Self(16);
    pub const B3: Self = Self(17);
    pub const C3: Self = Self(18);
    pub const D3: Self = Self(19);
    pub const E3: Self = Self(20);
    pub const F3: Self = Self(21);
    pub const G3: Self = Self(22);
    pub const H3: Self = Self(23);
    pub const A4: Self = Self(24);
    pub const B4: Self = Self(25);
    pub const C4: Self = Self(26);
    pub const D4: Self = Self(27);
    pub const E4: Self = Self(28);
    pub const F4: Self = Self(29);
    pub const G4: Self = Self(30);
    pub const H4: Self = Self(31);
    pub const A5: Self = Self(32);
    pub const B5: Self = Self(33);
    pub const C5: Self = Self(34);
    pub const D5: Self = Self(35);
    pub const E5: Self = Self(36);
    pub const F5: Self = Self(37);
    pub const G5: Self = Self(38);
    pub const H5: Self = Self(39);
    pub const A6: Self = Self(40);
    pub const B6: Self = Self(41);
    pub const C6: Self = Self(42);
    pub const D6: Self = Self(43);
    pub const E6: Self = Self(44);
    pub const F6: Self = Self(45);
    pub const G6: Self = Self(46);
    pub const H6: Self = Self(47);
    pub const A7: Self = Self(48);
    pub const B7: Self = Self(49);
    pub const C7: Self = Self(50);
    pub const D7: Self = Self(51);
    pub const E7: Self = Self(52);
    pub const F7: Self = Self(53);
    pub const G7: Self = Self(54);
    pub const H7: Self = Self(55);
    pub const A8: Self = Self(56);
    pub const B8: Self = Self(57);
    pub const C8: Self = Self(58);
    pub const D8: Self = Self(59);
    pub const E8: Self = Self(60);
    pub const F8: Self = Self(61);
    pub const G8: Self = Self(62);
    pub const H8: Self = Self(63);

    pub const ALL: &'static [Self] = &[
        Self::A1,
        Self::B1,
        Self::C1,
        Self::D1,
        Self::E1,
        Self::F1,
        Self::G1,
        Self::H1,
        Self::A2,
        Self::B2,
        Self::C2,
        Self::D2,
        Self::E2,
        Self::F2,
        Self::G2,
        Self::H2,
        Self::A3,
        Self::B3,
        Self::C3,
        Self::D3,
        Self::E3,
        Self::F3,
        Self::G3,
        Self::H3,
        Self::A4,
        Self::B4,
        Self::C4,
        Self::D4,
        Self::E4,
        Self::F4,
        Self::G4,
        Self::H4,
        Self::A5,
        Self::B5,
        Self::C5,
        Self::D5,
        Self::E5,
        Self::F5,
        Self::G5,
        Self::H5,
        Self::A6,
        Self::B6,
        Self::C6,
        Self::D6,
        Self::E6,
        Self::F6,
        Self::G6,
        Self::H6,
        Self::A7,
        Self::B7,
        Self::C7,
        Self::D7,
        Self::E7,
        Self::F7,
        Self::G7,
        Self::H7,
        Self::A8,
        Self::B8,
        Self::C8,
        Self::D8,
        Self::E8,
        Self::F8,
        Self::G8,
        Self::H8,
    ];

    pub fn file(self) -> File {
        File(self.0 % 8)
    }

    pub fn rank(self) -> Rank {
        Rank(self.0 / 8)
    }

    pub fn rank_file(self) -> (Rank, File) {
        (self.rank(), self.file())
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

impl From<(Rank, File)> for Square {
    fn from(value: (Rank, File)) -> Self {
        Self((value.0 .0) * 8 + (value.1 .0))
    }
}

impl From<(File, Rank)> for Square {
    fn from(value: (File, Rank)) -> Self {
        Self::from((value.1, value.0))
    }
}

impl From<Square> for Index {
    fn from(value: Square) -> Self {
        Index(value.0 as usize)
    }
}

impl Into<usize> for Square {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl From<usize> for Square {
    fn from(value: usize) -> Self {
        Self(value as u8)
    }
}

impl ArrayKey for Square {
    const COUNT: usize = 64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AttackMap {
    all: BitBoard,
    pawn: BitBoard,
}

// TODO: Move this!
fn generate_attacks(_piece: Piece, _square: Square, _occupancy: BitBoard) -> BitBoard {
    BitBoard::ZERO
}

impl AttackMap {
    fn from_occupancy(
        piece_occupancy: &ArrayMap<PieceIndex, BitBoard>,
        own_occupancy: BitBoard,
    ) -> Self {
        let mut all_attacks = BitBoard::ZERO;
        let mut pawn_attacks = BitBoard::ZERO;
        for color in Color::ALL {
            for piece in Piece::ALL {
                let piece_index = PieceIndex::new(*color, *piece);
                let mut occupancy = piece_occupancy[piece_index];
                while let Some(index) = occupancy.pop_lsb() {
                    let square = Square::from(index);
                    let attacks_this_piece = generate_attacks(*piece, square, own_occupancy);

                    all_attacks |= attacks_this_piece;
                    if *piece == Piece::Pawn {
                        pawn_attacks |= attacks_this_piece;
                    }
                }
            }
        }

        all_attacks &= !own_occupancy;
        pawn_attacks &= !own_occupancy;

        Self {
            all: all_attacks,
            pawn: pawn_attacks,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    occupancy: BitBoard,
    piece_occupancy: ArrayMap<PieceIndex, BitBoard>,
    colored_occupancy: ArrayMap<Color, BitBoard>,
    colored_attack_map: ArrayMap<Color, OnceCell<AttackMap>>,
}

impl Board {
    pub fn empty_map() -> ArrayMap<Square, PieceIndex> {
        ArrayMap::<Square, PieceIndex>::filled(PieceIndex::NONE)
    }

    pub fn new(piece_occupancy: ArrayMap<PieceIndex, BitBoard>) -> Self {
        let mut occupancy = BitBoard::ZERO;
        let mut colored_occupancy = ArrayMap::filled(BitBoard::ZERO);

        for color in Color::ALL {
            for piece in Piece::ALL {
                let piece_index = PieceIndex::new(*color, *piece);
                occupancy |= piece_occupancy[piece_index];
                colored_occupancy[*color] |= piece_occupancy[piece_index];
            }
        }

        Self {
            occupancy,
            piece_occupancy,
            colored_occupancy,
            colored_attack_map: ArrayMap::new([OnceCell::new(), OnceCell::new()]),
        }
    }

    pub fn occupancy(&self) -> BitBoard {
        self.occupancy
    }

    pub fn vacancy(&self) -> BitBoard {
        !self.occupancy
    }

    pub fn piece_occupancy(&self, piece_index: PieceIndex) -> BitBoard {
        self.piece_occupancy[piece_index]
    }

    pub fn colored_occupancy(&self, color: Color) -> BitBoard {
        self.colored_occupancy[color]
    }

    pub fn colored_attacks(&self, color: Color) -> BitBoard {
        self.attack_map(color).all
    }

    pub fn colored_pawn_attacks(&self, color: Color) -> BitBoard {
        self.attack_map(color).pawn
    }

    pub fn piece_at(&self, square: Square) -> Option<PieceIndex> {
        for color in Color::ALL {
            for piece in Piece::ALL {
                let piece_index = PieceIndex::new(*color, *piece);
                if self.piece_occupancy[piece_index][square] {
                    return Some(piece_index);
                }
            }
        }

        None
    }

    fn attack_map(&self, color: Color) -> &AttackMap {
        self.colored_attack_map[color].get_or_init(|| {
            AttackMap::from_occupancy(&self.piece_occupancy, self.colored_occupancy[color])
        })
    }
}

impl From<&ArrayMap<Square, PieceIndex>> for Board {
    fn from(arr: &ArrayMap<Square, PieceIndex>) -> Self {
        let mut piece_occupancy: ArrayMap<PieceIndex, BitBoard> = ArrayMap::default();
        for square in Square::ALL {
            let piece = arr[*square];
            piece_occupancy[piece].set(*square, piece.some());
        }

        Board::new(piece_occupancy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BitBoard(BitSet);

impl BitBoard {
    pub const ZERO: Self = Self(BitSet::ZERO);

    pub fn set<I>(&mut self, index: I, value: bool)
    where
        I: Into<Index>,
    {
        let index: Index = index.into();
        self.0.set(index.0, value);
    }

    pub fn pop_lsb(&mut self) -> Option<usize> {
        let bit = self.0.first_one()?;
        self.0.set(bit, false);
        Some(bit)
    }
}

impl<I> std::ops::Index<I> for BitBoard
where
    I: Into<Index>,
{
    type Output = bool;

    fn index(&self, index: I) -> &Self::Output {
        let index: Index = index.into();
        &self.0[index.0]
    }
}

impl Not for BitBoard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitOrAssign for BitBoard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAndAssign for BitBoard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

#[cfg(test)]
mod tests {

    use crate::game::Piece;

    use super::*;

    #[test]
    fn test_rank_indexes() {
        assert_eq!(Rank::ONE, Rank(0));
        assert_eq!(Rank::TWO, Rank(1));
        assert_eq!(Rank::THREE.to_string(), "3");
    }

    #[test]
    fn test_file_indexes() {
        assert_eq!(File::A, File(0));
        assert_eq!(File::B, File(1));
        assert_eq!(File::C.to_string(), "c");
    }

    #[test]
    fn test_square_indexes() {
        assert_eq!(Square::A1, Square(0));
        assert_eq!(Square::B1, Square(1));
        assert_eq!(Square::C5.to_string(), "c5");
    }

    #[test]
    fn test_bitboard_pop_lsb() {
        let mut board = BitBoard::ZERO;
        board.set(0, true);
        board.set(12, true);
        board.set(13, true);
        board.set(31, true);
        board.set(Square::C7, true);

        assert_eq!(board.pop_lsb(), Some(0));
        assert_eq!(board.pop_lsb(), Some(12));
        assert_eq!(board.pop_lsb(), Some(13));
        assert_eq!(board.pop_lsb(), Some(31));
        assert_eq!(board.pop_lsb(), Some(Square::C7.into()));
        assert_eq!(board.pop_lsb(), None);
    }

    #[test]
    fn test_all_locations() {
        assert_eq!(Square::ALL.len(), 64);
        assert_eq!(Square::ALL[0], Square::A1);
        assert_eq!(Square::ALL[1], Square::B1);
        assert_eq!(Square::ALL[11], Square::D2);
    }

    #[test]
    fn test_board_builder() {
        let mut map = Board::empty_map();
        map[Square::A1] = (Color::White, Piece::Rook).into();
        map[Square::B4] = (Color::White, Piece::Knight).into();
        map[Square::C7] = (Color::Black, Piece::Bishop).into();

        let board = Board::from(&map);
        assert!(board.colored_occupancy[Color::White][Square::A1]);
    }
}
