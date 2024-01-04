use weechess_core::{utils::ArrayMap, Color, Piece, PieceIndex};

use super::{Evaluation, StateVariation};

pub fn evaluate(v: &StateVariation<'_>, perspective: &Color, eval: &mut Evaluation, _: &mut bool) {
    for piece in Piece::ALL {
        let piece_index = PieceIndex::new(*perspective, *piece);
        let piece_worth = Evaluation::ONE_PAWN * PIECE_PAWN_WORTHS[*piece];
        let piece_count: i32 = v.piece_counts[piece_index] as i32;
        *eval += piece_worth * piece_count;
    }
}

pub const PIECE_PAWN_WORTHS: ArrayMap<Piece, f32> = ArrayMap::new([
    0.0,   // None
    1.0,   // Pawn
    3.0,   // Knight
    3.5,   // Bishop
    5.0,   // Rook
    9.0,   // Queen
    100.0, // King
]);
