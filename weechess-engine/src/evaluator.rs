use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul, Neg},
};

use weechess_core::{
    utils::ArrayMap, Color, MoveGenerationBuffer, MoveGenerator, Piece, PieceIndex, Square, State,
};

pub const PIECE_WORTHS: ArrayMap<Piece, i32> = ArrayMap::new([
    0,   // None
    1,   // Pawn
    3,   // Knight
    3,   // Bishop
    5,   // Rook
    9,   // Queen
    100, // King
]);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Evaluation(i32);

impl From<i32> for Evaluation {
    fn from(value: i32) -> Self {
        Evaluation(value)
    }
}

impl From<Evaluation> for i32 {
    fn from(value: Evaluation) -> Self {
        value.0
    }
}

impl Add<Evaluation> for Evaluation {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Evaluation(self.0 + rhs.0)
    }
}

impl AddAssign<Evaluation> for Evaluation {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Neg for Evaluation {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Evaluation(-self.0)
    }
}

impl Mul<i32> for Evaluation {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Evaluation(self.0 * rhs)
    }
}

impl Evaluation {
    pub const EVEN: Evaluation = Evaluation(0);
    pub const ONE_PAWN: Evaluation = Evaluation(100);
    pub const POS_INF: Evaluation = Evaluation(Self::ONE_PAWN.0 * 100);
    pub const NEG_INF: Evaluation = Evaluation(Self::ONE_PAWN.0 * -100);

    pub fn mate_in(_ply: usize) -> Evaluation {
        Evaluation::POS_INF
    }

    pub fn is_terminal(self) -> bool {
        self <= Self::NEG_INF || self >= Self::POS_INF
    }

    pub fn cp(self) -> Centipawn {
        Centipawn(self.0 as f32)
    }
}

impl Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self >= Self::EVEN {
            write!(f, "+")?;
        }

        write!(f, "{:.1}", (self.0 as f32) / (Self::ONE_PAWN.0 as f32))
    }
}

pub struct Centipawn(f32);

impl Display for Centipawn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 0.0 {
            write!(f, "+")?;
        }

        write!(f, "{:.1}", self.0)
    }
}

pub struct Evaluator;

impl Evaluator {
    pub fn new() -> Self {
        Evaluator
    }

    pub fn estimate(&self, state: &State, perspective: Color) -> Evaluation {
        // Dumb estimate: add up piece worths
        let mut evaluation = Evaluation::EVEN;
        for color in Color::ALL {
            let mutiplier = if *color == perspective { 1 } else { -1 };

            for piece in Piece::ALL {
                let piece_index = PieceIndex::new(*color, *piece);
                let piece_worth = Evaluation::ONE_PAWN * PIECE_WORTHS[*piece];
                let piece_count = state.board().piece_occupancy(piece_index).count_ones() as i32;
                evaluation += Evaluation::from(piece_worth) * mutiplier * piece_count;
            }
        }

        evaluation
    }

    pub fn evaluate(&self, state: &State, perspective: Color) -> Evaluation {
        let move_generator = MoveGenerator;
        let mut move_buffer = MoveGenerationBuffer::new();
        move_generator.compute_legal_moves_into(state, &mut move_buffer);

        if move_buffer.legal_moves.is_empty() && state.is_check() {
            return if state.turn_to_move() == perspective {
                Evaluation::NEG_INF
            } else {
                Evaluation::POS_INF
            };
        } else if move_buffer.legal_moves.is_empty() {
            return Evaluation::EVEN;
        }

        let end_game_weight = {
            let white_queens = state
                .board()
                .piece_occupancy(PieceIndex::new(Color::White, Piece::Queen))
                .count_ones();
            let black_queens = state
                .board()
                .piece_occupancy(PieceIndex::new(Color::Black, Piece::Queen))
                .count_ones();
            let white_rooks = state
                .board()
                .piece_occupancy(PieceIndex::new(Color::White, Piece::Rook))
                .count_ones();
            let black_rooks = state
                .board()
                .piece_occupancy(PieceIndex::new(Color::Black, Piece::Rook))
                .count_ones();

            let white_queens = white_queens as f32;
            let black_queens = black_queens as f32;
            let white_rooks = white_rooks as f32;
            let black_rooks = black_rooks as f32;

            1.0 - (white_queens + black_queens + white_rooks + black_rooks) / 6.0
        };

        let mut eval = self.estimate(state, perspective);

        eval += {
            // Piece square tables
            let mut eval = 0.0;
            for color in Color::ALL {
                let mut color_eval = 0.0;
                for piece in Piece::ALL {
                    let piece_index = PieceIndex::new(*color, *piece);
                    let piece_occupancy = state.board().piece_occupancy(piece_index);
                    for square in piece_occupancy.iter_ones() {
                        let square = if *color == Color::White {
                            Square::from(square)
                        } else {
                            Square::from(square).flip_rank()
                        };

                        let index = square.white_at_bottom_index();
                        let e1 = *weights::PIECE_SQUARE_MAP[*piece][0].index(index) as f32;
                        let e2 = *weights::PIECE_SQUARE_MAP[*piece][1].index(index) as f32;

                        // Lerp between e1 and e2 by end_game_weight
                        color_eval += (e2 - e1) * end_game_weight + e1;
                    }
                }

                eval += color_eval * if *color == perspective { 1.0 } else { -1.0 };
            }

            Evaluation((eval * 0.1) as i32)
        };

        eval
    }
}

mod weights {
    use weechess_core::{utils::ArrayMap, Piece, Square};

    pub const PIECE_SQUARE_MAP: ArrayMap<Piece, [ArrayMap<Square, i32>; 2]> = ArrayMap::new([
        [ZERO_MAP, ZERO_MAP],
        [PAWN_MAP, PAWN_MAP],
        [KNIGHT_MAP, KNIGHT_MAP],
        [BISHOP_MAP, BISHOP_MAP],
        [ROOK_MAP, ROOK_MAP],
        [QUEEN_MAP, QUEEN_MAP],
        [KING_MIDDLE_GAME_MAP, KING_END_GAME_MAP],
    ]);

    #[rustfmt::skip]
    pub const ZERO_MAP: ArrayMap<Square, i32> = ArrayMap::new([0; 64]);

    #[rustfmt::skip]
    pub const PAWN_MAP: ArrayMap<Square, i32> = ArrayMap::new([
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
    pub const KNIGHT_MAP: ArrayMap<Square, i32> = ArrayMap::new([
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
    pub const BISHOP_MAP: ArrayMap<Square, i32> = ArrayMap::new([
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
    pub const ROOK_MAP: ArrayMap<Square, i32> = ArrayMap::new([
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
    pub const QUEEN_MAP: ArrayMap<Square, i32> = ArrayMap::new([
        -20,-10,-10, -5, -5,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5,  5,  5,  5,  0,-10,
         -5,  0,  5,  5,  5,  5,  0, -5,
          0,  0,  5,  5,  5,  5,  0, -5,
        -10,  5,  5,  5,  5,  5,  0,-10,
        -10,  0,  5,  0,  0,  0,  0,-10,
        -20,-10,-10, -5, -5,-10,-10,-20
    ]);

    #[rustfmt::skip]
    pub const KING_MIDDLE_GAME_MAP: ArrayMap<Square, i32> = ArrayMap::new([
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
    pub const KING_END_GAME_MAP: ArrayMap<Square, i32> = ArrayMap::new([
        -50,-40,-30,-20,-20,-30,-40,-50,
        -30,-20,-10,  0,  0,-10,-20,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-30,  0,  0,  0,  0,-30,-30,
        -50,-30,-30,-30,-30,-30,-30,-50
    ]);
}

#[cfg(test)]
mod tests {
    use weechess_core::MoveQuery;

    use super::*;

    #[test]
    fn test_empty() {
        let evaluator = Evaluator::new();
        let game_state = State::default();

        assert_eq!(
            evaluator.evaluate(&game_state, Color::White),
            Evaluation::EVEN
        );

        assert_eq!(
            evaluator.evaluate(&game_state, Color::Black),
            Evaluation::EVEN
        );
    }

    #[test]
    fn test_pawn_map() {
        let evaluator = Evaluator::new();
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
