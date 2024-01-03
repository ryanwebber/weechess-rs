use weechess_core::{utils::ArrayMap, Color, Piece, PieceIndex, Square};

use super::{Evaluation, StateVariation};

pub fn evaluate(v: &StateVariation<'_>, perspective: &Color, eval: &mut Evaluation, _: &mut bool) {
    *eval += {
        let mut eval = 0.0;
        for piece in Piece::ALL {
            let piece_index = PieceIndex::new(*perspective, *piece);
            let piece_occupancy = v.board().piece_occupancy(piece_index);
            for square in piece_occupancy.iter_ones() {
                let square = if *perspective == Color::White {
                    Square::from(square)
                } else {
                    Square::from(square).flip_rank()
                };

                let index = square.white_at_bottom_index();
                let e1 = *PIECE_SQUARE_MAP[*piece][0].index(index) as f32;
                let e2 = *PIECE_SQUARE_MAP[*piece][1].index(index) as f32;

                // Lerp between e1 and e2 by end_game_weight
                eval += (e2 - e1) * v.end_game_weight + e1;
            }
        }

        Evaluation(eval as i32)
    }
}

const PIECE_SQUARE_MAP: ArrayMap<Piece, [ArrayMap<Square, i32>; 2]> = ArrayMap::new([
    [ZERO_MAP, ZERO_MAP],
    [PAWN_MAP, PAWN_MAP],
    [KNIGHT_MAP, KNIGHT_MAP],
    [BISHOP_MAP, BISHOP_MAP],
    [ROOK_MAP, ROOK_MAP],
    [QUEEN_MAP, QUEEN_MAP],
    [KING_MIDDLE_GAME_MAP, KING_END_GAME_MAP],
]);

#[rustfmt::skip]
const ZERO_MAP: ArrayMap<Square, i32> = ArrayMap::new([0; 64]);

#[rustfmt::skip]
const PAWN_MAP: ArrayMap<Square, i32> = ArrayMap::new([
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
]);

#[rustfmt::skip]
const KNIGHT_MAP: ArrayMap<Square, i32> = ArrayMap::new([
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
]);

#[rustfmt::skip]
const BISHOP_MAP: ArrayMap<Square, i32> = ArrayMap::new([
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
]);

#[rustfmt::skip]
const ROOK_MAP: ArrayMap<Square, i32> = ArrayMap::new([
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0,
]);

#[rustfmt::skip]
const QUEEN_MAP: ArrayMap<Square, i32> = ArrayMap::new([
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
]);

#[rustfmt::skip]
const KING_MIDDLE_GAME_MAP: ArrayMap<Square, i32> = ArrayMap::new([
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20,
]);

#[rustfmt::skip]
const KING_END_GAME_MAP: ArrayMap<Square, i32> = ArrayMap::new([
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50,
]);

#[cfg(test)]
mod tests {
    use crate::eval::Evaluator;

    use super::*;
    use weechess_core::{MoveQuery, State};

    #[test]
    fn test_pawn_map() {
        let evaluator = Evaluator::just(&[(1.0, super::evaluate)]);
        let state1 = State::default();
        let state2 = State::by_performing_moves(
            &state1,
            &[MoveQuery::by_moving_from_to(Square::D2, Square::D4)],
        )
        .unwrap();

        let e1 = evaluator.evaluate(&state1, Color::White);
        let e2 = evaluator.evaluate(&state2, Color::White);
        assert!(e2 > e1);

        let e1 = evaluator.evaluate(&state1, Color::Black);
        let e2 = evaluator.evaluate(&state2, Color::Black);
        assert!(e2 < e1);
    }
}
