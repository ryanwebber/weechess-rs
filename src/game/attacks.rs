use super::{BitBoard, Piece, Square};

pub struct AttackGenerator;

impl AttackGenerator {
    pub fn compute(&self, _piece: Piece, _square: Square, _occupancy: BitBoard) -> BitBoard {
        BitBoard::ZERO
    }
}
