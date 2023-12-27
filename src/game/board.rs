use std::{
    cell::OnceCell,
    fmt::Display,
    ops::{Add, BitAnd, BitAndAssign, BitOr, BitOrAssign, Not},
};

use super::{ArrayKey, ArrayMap, AttackGenerator, Color, Index, Piece, PieceIndex, FILE_MASKS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    King,
    Queen,
}

impl Side {
    pub const ALL: &'static [Self] = &[Self::King, Self::Queen];
}

impl From<Side> for Index {
    fn from(value: Side) -> Self {
        Index(match value {
            Side::King => 0,
            Side::Queen => 1,
        })
    }
}

impl TryFrom<usize> for Side {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::King),
            1 => Ok(Self::Queen),
            _ => Err(()),
        }
    }
}

impl ArrayKey for Side {
    const COUNT: usize = 2;
}

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

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn from_char(c: char) -> Option<Self> {
        let c = c.to_ascii_uppercase();
        if c < 'A' || c > 'H' {
            None
        } else {
            Some(Self(c as u8 - 'A' as u8))
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

    pub fn index(self) -> usize {
        self.0 as usize
    }

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

    pub fn abs_distance_to(self, other: Self) -> u8 {
        debug_assert!(self.0 < 8);
        debug_assert!(other.0 < 8);
        (self.0 as i8 - other.0 as i8).abs() as u8
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
pub struct Offset {
    pub file: i8,
    pub rank: i8,
}

impl Offset {
    pub const NORTH: Self = Self { file: 0, rank: 1 };
    pub const SOUTH: Self = Self { file: 0, rank: -1 };
    pub const EAST: Self = Self { file: 1, rank: 0 };
    pub const WEST: Self = Self { file: -1, rank: 0 };
}

impl Add for Offset {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            file: self.file + rhs.file,
            rank: self.rank + rhs.rank,
        }
    }
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

    pub fn offset(self, offset: Offset) -> Option<Self> {
        let file = self.file().0 as i8 + offset.file;
        let rank = self.rank().0 as i8 + offset.rank;
        if file < 0 || file > 7 || rank < 0 || rank > 7 {
            None
        } else {
            Some(Self::from((Rank(rank as u8), File(file as u8))))
        }
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

impl Into<u8> for Square {
    fn into(self) -> u8 {
        self.0
    }
}

impl From<u8> for Square {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl Into<u32> for Square {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl From<u32> for Square {
    fn from(value: u32) -> Self {
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

impl AttackMap {
    fn from_occupancy(
        color: Color,
        piece_occupancy: &ArrayMap<PieceIndex, BitBoard>,
        shared_occupancy: BitBoard,
        own_occupancy: BitBoard,
    ) -> Self {
        let mut all_attacks = BitBoard::ZERO;
        let mut pawn_attacks = BitBoard::ZERO;
        for piece in Piece::ALL {
            let piece_index = PieceIndex::new(color, *piece);
            let mut occupancy = piece_occupancy[piece_index];
            while let Some(square) = occupancy.pop() {
                let attacks_this_piece = AttackGenerator::compute(
                    &AttackGenerator,
                    piece_index,
                    square,
                    shared_occupancy,
                );

                all_attacks |= attacks_this_piece;
                if *piece == Piece::Pawn {
                    pawn_attacks |= attacks_this_piece;
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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct BitBoard(u64);

impl BitBoard {
    pub const ZERO: Self = Self(0);
    pub const BIT_COUNT: u32 = u64::BITS;

    #[inline]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[inline]
    pub const fn just(square: Square) -> Self {
        Self(1u64 << square.0)
    }

    #[inline]
    pub fn any(self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn none(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn first_one(self) -> Option<u32> {
        let z = self.0.trailing_zeros();
        if z == Self::BIT_COUNT {
            None
        } else {
            Some(z)
        }
    }

    #[inline]
    pub fn last_one(self) -> Option<u32> {
        let z = self.0.leading_zeros();
        if z == Self::BIT_COUNT {
            None
        } else {
            Some(Self::BIT_COUNT - z - 1)
        }
    }

    #[inline]
    pub fn iter_ones(self) -> impl Iterator<Item = u32> {
        BitIterator(self)
    }

    #[inline]
    pub fn set(&mut self, square: Square, value: bool) {
        self.set_raw(square.into(), value);
    }

    #[inline]
    pub fn set_raw(&mut self, bit: u32, value: bool) {
        if value {
            self.0 |= 1u64 << bit;
        } else {
            self.0 &= !(1u64 << bit);
        }
    }

    #[inline]
    pub fn test(self, square: Square) -> bool {
        let bit: u32 = square.into();
        self.test_raw(bit)
    }

    #[inline]
    pub fn test_raw(self, bit: u32) -> bool {
        (self.0 & (1u64 << bit)) != 0
    }

    #[inline]
    pub fn pop(&mut self) -> Option<Square> {
        let Some(bit) = self.first_one() else {
            return None;
        };

        self.set_raw(bit, false);
        Some(Square::from(bit as u8))
    }

    #[inline]
    pub fn shift(self, offset: Offset) -> Self {
        let mut bb = self;
        if offset.rank > 0 {
            bb = BitBoard(bb.0 << offset.rank * 8);
        } else if offset.rank < 0 {
            bb = BitBoard(bb.0 >> -offset.rank * 8);
        }

        if offset.file > 0 {
            for _ in 0..offset.file {
                bb = BitBoard((bb & !FILE_MASKS[File::H]).0 << 1);
            }
        } else if offset.file < 0 {
            for _ in 0..-offset.file {
                bb = BitBoard((bb & !FILE_MASKS[File::A]).0 >> 1);
            }
        }

        bb
    }
}

impl Not for BitBoard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitOr for BitBoard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for BitBoard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for BitBoard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for BitBoard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Into<u64> for BitBoard {
    fn into(self) -> u64 {
        self.0
    }
}

impl From<u64> for BitBoard {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

pub struct BitIterator(BitBoard);

impl Iterator for BitIterator {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(place) = self.0.first_one() else {
            return None;
        };

        self.0 .0 &= !(1u64 << place);
        Some(place)
    }
}

impl std::fmt::Debug for BitBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for rank in Rank::ALL.iter().rev() {
            for file in File::ALL.iter() {
                let square = Square::from((*rank, *file));
                write!(f, "{}", if self.test(square) { '1' } else { '.' })?;
            }

            writeln!(f)?;
        }

        Ok(())
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

    pub fn piece_at(&self, square: Square) -> Option<PieceIndex> {
        for color in Color::ALL {
            for piece in Piece::ALL {
                let piece_index = PieceIndex::new(*color, *piece);
                if self.piece_occupancy[piece_index].test(square) {
                    return Some(piece_index);
                }
            }
        }

        None
    }

    pub fn piece_map(&self) -> &ArrayMap<PieceIndex, BitBoard> {
        &self.piece_occupancy
    }

    pub fn pieces(&self) -> impl Iterator<Item = (Square, PieceIndex)> + '_ {
        Square::ALL.iter().filter_map(move |square| {
            if let Some(piece) = self.piece_at(*square) {
                Some((*square, piece))
            } else {
                None
            }
        })
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

    fn attack_map(&self, color: Color) -> &AttackMap {
        self.colored_attack_map[color].get_or_init(|| {
            AttackMap::from_occupancy(
                color,
                &self.piece_occupancy,
                self.occupancy,
                self.colored_occupancy[color],
            )
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

impl From<&Board> for ArrayMap<Square, PieceIndex> {
    fn from(board: &Board) -> Self {
        let mut arr = ArrayMap::filled(PieceIndex::NONE);
        for square in Square::ALL {
            if let Some(piece) = board.piece_at(*square) {
                arr[*square] = piece;
            }
        }

        arr
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
    fn test_bitboard_pop() {
        let mut board = BitBoard::ZERO;
        board.set(Square::from(0 as u8), true);
        board.set(Square::from(12 as u8), true);
        board.set(Square::from(13 as u8), true);
        board.set(Square::from(31 as u8), true);
        board.set(Square::C7, true);

        assert_eq!(board.pop(), Some(Square::from(0 as u8)));
        assert_eq!(board.pop(), Some(Square::from(12 as u8)));
        assert_eq!(board.pop(), Some(Square::from(13 as u8)));
        assert_eq!(board.pop(), Some(Square::from(31 as u8)));
        assert_eq!(board.pop(), Some(Square::C7.into()));
        assert_eq!(board.pop(), None);
    }

    #[test]
    fn test_bitboard_ones() {
        let bb = BitBoard::new(0b00100010);
        assert_eq!(bb.first_one(), Some(1));
        assert_eq!(bb.last_one(), Some(5));
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
        assert!(board.colored_occupancy[Color::White].test(Square::A1));
    }

    #[test]
    fn test_board_shifts() {
        let square = Square::A4;
        assert_eq!(square.offset(Offset::NORTH), Some(Square::A5));
        assert_eq!(square.offset(Offset::SOUTH), Some(Square::A3));
        assert_eq!(square.offset(Offset::EAST), Some(Square::B4));
        assert_eq!(
            square.offset(Offset::NORTH + Offset::EAST),
            Some(Square::B5)
        );

        assert_eq!(square.offset(Offset::WEST), None);
        assert_eq!(square.offset(Offset::NORTH + Offset::WEST), None);
    }
}
