use self::compact::BitSetExt;

use super::{common, Color, Piece, PieceIndex, Side, Square, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveQuery {
    FromTo(Square, Square),
}

impl MoveQuery {
    pub fn test(&self, m: &Move) -> bool {
        match self {
            MoveQuery::FromTo(from, to) => m.origin() == *from && m.destination() == *to,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move(compact::BitSet);

impl Move {
    pub const NULL: Move = Move(compact::BitSet::ZERO);

    pub fn by_moving(piece: PieceIndex, origin: Square, dest: Square) -> Self {
        let mut bits = compact::BitSet::ZERO;
        bits.set_piece(piece.piece());
        bits.set_origin(origin);
        bits.set_dest(dest);
        bits.set_color(piece.color() == Color::White);

        if piece.piece() == Piece::Pawn && origin.rank().abs_distance_to(dest.rank()) > 1 {
            bits.set_double_pawn(true);
        }

        Self(bits)
    }

    pub fn by_capturing(piece: PieceIndex, origin: Square, dest: Square, capturing: Piece) -> Self {
        let mut this = Self::by_moving(piece, origin, dest);
        this.0.set_capture(Some(capturing));
        this
    }

    pub fn by_promoting(piece: PieceIndex, origin: Square, dest: Square, promotion: Piece) -> Self {
        let mut this = Self::by_moving(piece, origin, dest);
        this.0.set_promotion(Some(promotion));
        this
    }

    pub fn by_capture_promoting(
        piece: PieceIndex,
        origin: Square,
        dest: Square,
        capturing: Piece,
        promotion: Piece,
    ) -> Self {
        let mut this = Self::by_moving(piece, origin, dest);
        this.0.set_capture(Some(capturing));
        this.0.set_promotion(Some(promotion));
        this
    }

    pub fn by_en_passant(piece: PieceIndex, origin: Square, dest: Square) -> Self {
        let mut this = Self::by_moving(piece, origin, dest);
        this.0.set_en_passant(true);
        this.0.set_capture(Some(Piece::Pawn));
        this
    }

    pub fn by_castling(color: Color, side: Side) -> Self {
        let origin = common::KING_ORIGINS[color];
        let dest = common::CASTLE_DESTS[color][side];
        let mut this = Self::by_moving(PieceIndex::new(color, Piece::King), origin, dest);
        this.0.set_castle_queenside(side == Side::Queen);
        this.0.set_castle_kingside(side == Side::King);
        this
    }

    pub fn origin(&self) -> Square {
        self.0.origin()
    }

    pub fn destination(&self) -> Square {
        self.0.dest()
    }

    pub fn is_capture(&self) -> bool {
        self.capture().is_some()
    }

    pub fn is_promotion(&self) -> bool {
        self.promotion().is_some()
    }

    pub fn is_en_passant(&self) -> bool {
        self.0.en_passant()
    }

    pub fn is_double_pawn(&self) -> bool {
        self.0.double_pawn()
    }

    pub fn is_any_castle(&self) -> bool {
        self.is_castle(Side::Queen) || self.is_castle(Side::King)
    }

    pub fn is_castle(&self, side: Side) -> bool {
        self.castle_side() == Some(side)
    }

    pub fn castle_side(&self) -> Option<Side> {
        if self.0.castle_queenside() {
            Some(Side::Queen)
        } else if self.0.castle_kingside() {
            Some(Side::King)
        } else {
            None
        }
    }

    pub fn color(&self) -> Color {
        if self.0.color() {
            Color::White
        } else {
            Color::Black
        }
    }

    pub fn piece(&self) -> Piece {
        self.0.piece()
    }

    pub fn capture(&self) -> Option<Piece> {
        self.0.capture()
    }

    pub fn promotion(&self) -> Option<Piece> {
        self.0.promotion()
    }

    pub fn resulting_piece(&self) -> Piece {
        self.promotion().unwrap_or(self.piece())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveResult(pub Move, pub State);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveSet(Vec<MoveResult>);

impl MoveSet {
    pub fn empty() -> Self {
        MoveSet(Vec::new())
    }

    pub fn new(moves: Vec<MoveResult>) -> Self {
        Self(moves)
    }

    pub fn moves(&self) -> &[MoveResult] {
        &self.0
    }

    pub fn find(&self, query: &MoveQuery) -> Option<MoveResult> {
        self.0.iter().find(|m| query.test(&m.0)).cloned()
    }

    pub fn filter<'a>(&'a self, query: MoveQuery) -> impl Iterator<Item = &'a MoveResult> {
        self.0.iter().filter(move |m| query.test(&m.0))
    }
}

impl Into<Vec<MoveResult>> for MoveSet {
    fn into(self) -> Vec<MoveResult> {
        self.0
    }
}

impl From<Vec<MoveResult>> for MoveSet {
    fn from(moves: Vec<MoveResult>) -> Self {
        Self(moves)
    }
}

mod compact {
    use std::ops::Range;

    use bitvec::field::BitField;
    use num_enum::TryFromPrimitive;

    use crate::game::{Piece, Square};

    pub type BitSet = bitvec::BitArr!(for 32);

    const PIECE_BIT_FIELD: Range<usize> = 0..4;
    const ORIGIN_BIT_FIELD: Range<usize> = 4..10;
    const DEST_BIT_FIELD: Range<usize> = 10..16;
    const CAPTURE_BIT_FIELD: Range<usize> = 16..20;
    const PROMOTION_BIT_FIELD: Range<usize> = 20..24;
    const EN_PASSANT_BIT: usize = 24;
    const DOUBLE_PAWN_BIT: usize = 25;
    const CASTLE_QUEENSIDE_BIT: usize = 26;
    const CASTLE_KINGSIDE_BIT: usize = 27;
    const COLOR_BIT: usize = 28;

    pub trait BitSetExt {
        fn piece(&self) -> Piece;
        fn set_piece(&mut self, piece: Piece);

        fn origin(&self) -> Square;
        fn set_origin(&mut self, origin: Square);

        fn dest(&self) -> Square;
        fn set_dest(&mut self, dest: Square);

        fn capture(&self) -> Option<Piece>;
        fn set_capture(&mut self, capture: Option<Piece>);

        fn promotion(&self) -> Option<Piece>;
        fn set_promotion(&mut self, promotion: Option<Piece>);

        fn en_passant(&self) -> bool;
        fn set_en_passant(&mut self, en_passant: bool);

        fn double_pawn(&self) -> bool;
        fn set_double_pawn(&mut self, double_pawn: bool);

        fn castle_queenside(&self) -> bool;
        fn set_castle_queenside(&mut self, castle_queenside: bool);

        fn castle_kingside(&self) -> bool;
        fn set_castle_kingside(&mut self, castle_kingside: bool);

        fn color(&self) -> bool;
        fn set_color(&mut self, color: bool);
    }

    impl BitSetExt for BitSet {
        fn piece(&self) -> Piece {
            let piece: u8 = self[PIECE_BIT_FIELD].load();
            Piece::try_from_primitive(piece).unwrap()
        }

        fn set_piece(&mut self, piece: Piece) {
            let value: u8 = piece.into();
            self[PIECE_BIT_FIELD].store(value);
        }

        fn origin(&self) -> Square {
            let origin: u8 = self[ORIGIN_BIT_FIELD].load();
            Square::from(origin)
        }

        fn set_origin(&mut self, origin: Square) {
            let value: u8 = origin.into();
            self[ORIGIN_BIT_FIELD].store(value);
        }

        fn dest(&self) -> Square {
            let dest: u8 = self[DEST_BIT_FIELD].load();
            Square::from(dest)
        }

        fn set_dest(&mut self, dest: Square) {
            let value: u8 = dest.into();
            self[DEST_BIT_FIELD].store(value);
        }

        fn capture(&self) -> Option<Piece> {
            let capture: u8 = self[CAPTURE_BIT_FIELD].load();
            if capture == 0 {
                None
            } else {
                Some(Piece::try_from_primitive(capture).unwrap())
            }
        }

        fn set_capture(&mut self, capture: Option<Piece>) {
            let value: u8 = capture.map(|p| p.into()).unwrap_or(0);
            self[CAPTURE_BIT_FIELD].store(value);
        }

        fn promotion(&self) -> Option<Piece> {
            let promotion: u8 = self[PROMOTION_BIT_FIELD].load();
            if promotion == 0 {
                None
            } else {
                Some(Piece::try_from_primitive(promotion).unwrap())
            }
        }

        fn set_promotion(&mut self, promotion: Option<Piece>) {
            let value: u8 = promotion.map(|p| p.into()).unwrap_or(0);
            self[PROMOTION_BIT_FIELD].store(value);
        }

        fn en_passant(&self) -> bool {
            self[EN_PASSANT_BIT]
        }

        fn set_en_passant(&mut self, en_passant: bool) {
            self.set(EN_PASSANT_BIT, en_passant);
        }

        fn double_pawn(&self) -> bool {
            self[DOUBLE_PAWN_BIT]
        }

        fn set_double_pawn(&mut self, double_pawn: bool) {
            self.set(DOUBLE_PAWN_BIT, double_pawn);
        }

        fn castle_queenside(&self) -> bool {
            self[CASTLE_QUEENSIDE_BIT]
        }

        fn set_castle_queenside(&mut self, castle_queenside: bool) {
            self.set(CASTLE_QUEENSIDE_BIT, castle_queenside);
        }

        fn castle_kingside(&self) -> bool {
            self[CASTLE_KINGSIDE_BIT]
        }

        fn set_castle_kingside(&mut self, castle_kingside: bool) {
            self.set(CASTLE_KINGSIDE_BIT, castle_kingside);
        }

        fn color(&self) -> bool {
            self[COLOR_BIT]
        }

        fn set_color(&mut self, color: bool) {
            self.set(COLOR_BIT, color);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::game::{Color, Piece, PieceIndex, Square};

    use super::Move;

    #[test]
    fn test_simple_move() {
        let m = Move::by_moving(
            PieceIndex::new(Color::White, Piece::Pawn),
            Square::A2,
            Square::A3,
        );

        assert_eq!(m.origin(), Square::A2);
        assert_eq!(m.destination(), Square::A3);
        assert_eq!(m.color(), Color::White);
        assert_eq!(m.piece(), Piece::Pawn);
        assert_eq!(m.capture(), None);
        assert_eq!(m.promotion(), None);
        assert!(!m.is_capture());
        assert!(!m.is_promotion());
        assert!(!m.is_en_passant());
        assert!(!m.is_double_pawn());
        assert!(!m.is_any_castle());
    }

    #[test]
    fn test_castle_move() {
        let m = Move::by_castling(Color::White, crate::game::Side::Queen);

        assert_eq!(m.origin(), Square::E1);
        assert_eq!(m.destination(), Square::C1);
        assert_eq!(m.color(), Color::White);
        assert_eq!(m.piece(), Piece::King);
        assert_eq!(m.capture(), None);
        assert_eq!(m.promotion(), None);
        assert!(!m.is_capture());
        assert!(!m.is_promotion());
        assert!(!m.is_en_passant());
        assert!(!m.is_double_pawn());
        assert!(m.is_any_castle());
        assert!(m.is_castle(crate::game::Side::Queen));
        assert!(!m.is_castle(crate::game::Side::King));
    }

    #[test]
    fn test_capture_move() {
        let m = Move::by_capturing(
            PieceIndex::new(Color::White, Piece::Pawn),
            Square::A2,
            Square::B3,
            Piece::Knight,
        );

        assert_eq!(m.origin(), Square::A2);
        assert_eq!(m.destination(), Square::B3);
        assert_eq!(m.color(), Color::White);
        assert_eq!(m.piece(), Piece::Pawn);
        assert_eq!(m.capture(), Some(Piece::Knight));
        assert_eq!(m.promotion(), None);
        assert!(m.is_capture());
        assert!(!m.is_promotion());
        assert!(!m.is_en_passant());
        assert!(!m.is_double_pawn());
        assert!(!m.is_any_castle());
    }

    #[test]
    fn test_promotion_move() {
        let m = Move::by_promoting(
            PieceIndex::new(Color::White, Piece::Pawn),
            Square::A7,
            Square::A8,
            Piece::Queen,
        );

        assert_eq!(m.origin(), Square::A7);
        assert_eq!(m.destination(), Square::A8);
        assert_eq!(m.color(), Color::White);
        assert_eq!(m.piece(), Piece::Pawn);
        assert_eq!(m.capture(), None);
        assert_eq!(m.promotion(), Some(Piece::Queen));
        assert!(!m.is_capture());
        assert!(m.is_promotion());
        assert!(!m.is_en_passant());
        assert!(!m.is_double_pawn());
        assert!(!m.is_any_castle());
    }

    #[test]
    fn test_capture_promotion_move() {
        let m = Move::by_capture_promoting(
            PieceIndex::new(Color::White, Piece::Pawn),
            Square::A7,
            Square::B8,
            Piece::Knight,
            Piece::Queen,
        );

        assert_eq!(m.origin(), Square::A7);
        assert_eq!(m.destination(), Square::B8);
        assert_eq!(m.color(), Color::White);
        assert_eq!(m.piece(), Piece::Pawn);
        assert_eq!(m.capture(), Some(Piece::Knight));
        assert_eq!(m.promotion(), Some(Piece::Queen));
        assert!(m.is_capture());
        assert!(m.is_promotion());
        assert!(!m.is_en_passant());
        assert!(!m.is_double_pawn());
        assert!(!m.is_any_castle());
    }

    #[test]
    fn test_en_passant_move() {
        let m = Move::by_en_passant(
            PieceIndex::new(Color::White, Piece::Pawn),
            Square::A5,
            Square::B6,
        );

        assert_eq!(m.origin(), Square::A5);
        assert_eq!(m.destination(), Square::B6);
        assert_eq!(m.color(), Color::White);
        assert_eq!(m.piece(), Piece::Pawn);
        assert_eq!(m.promotion(), None);
        assert_eq!(m.capture(), Some(Piece::Pawn));
        assert!(m.is_capture());
        assert!(!m.is_promotion());
        assert!(m.is_en_passant());
        assert!(!m.is_double_pawn());
        assert!(!m.is_any_castle());
    }

    #[test]
    fn test_double_pawn_move() {
        let m = Move::by_moving(
            PieceIndex::new(Color::White, Piece::Pawn),
            Square::A2,
            Square::A4,
        );

        assert_eq!(m.origin(), Square::A2);
        assert_eq!(m.destination(), Square::A4);
        assert_eq!(m.color(), Color::White);
        assert_eq!(m.piece(), Piece::Pawn);
        assert_eq!(m.capture(), None);
        assert_eq!(m.promotion(), None);
        assert!(!m.is_capture());
        assert!(!m.is_promotion());
        assert!(!m.is_en_passant());
        assert!(m.is_double_pawn());
        assert!(!m.is_any_castle());
    }
}
