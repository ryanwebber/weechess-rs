use weechess_core::{utils::ArrayMap, Color, Piece, PieceIndex, State};

use super::Evaluation;

pub fn estimate(v: &State, perspective: &Color) -> Evaluation {
    let mut eval = Evaluation::EVEN;
    for color in Color::ALL {
        let mutiplier = if color == perspective { 1 } else { -1 };
        for piece in Piece::ALL {
            let piece_index = PieceIndex::new(*color, *piece);
            let piece_worth = Evaluation::ONE_PAWN * PIECE_WORTHS[*piece];
            let piece_count = v.board().piece_occupancy(piece_index).count_ones() as i32;
            eval += Evaluation::from(piece_worth) * mutiplier * piece_count;
        }
    }

    eval
}

const PIECE_WORTHS: ArrayMap<Piece, i32> = ArrayMap::new([
    0,   // None
    1,   // Pawn
    3,   // Knight
    3,   // Bishop
    5,   // Rook
    9,   // Queen
    100, // King
]);
