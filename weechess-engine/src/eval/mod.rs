use std::{
    fmt::Display,
    ops::{Add, AddAssign, Deref, Mul, Neg, Sub, SubAssign},
};

use weechess_core::{
    utils::ArrayMap, AttackGenerator, BitBoard, Color, Move, MoveGenerator, Piece, PieceIndex,
    State,
};

mod evaluate_bad_pawns;
mod evaluate_force_king_to_edge;
mod evaluate_piece_squares;
mod evaluate_piece_worths;

pub use evaluate_piece_worths::PIECE_PAWN_WORTHS;

type EvaluationFunction =
    fn(v: &StateVariation<'_>, perspective: &Color, eval: &mut Evaluation, stop: &mut bool);

const EVALUATORS: &'static [(f32, EvaluationFunction)] = &[
    (1.0, evaluate_piece_worths::evaluate),
    (0.8, evaluate_piece_squares::evaluate),
    (1.0, evaluate_force_king_to_edge::evaluate),
    (0.2, evaluate_bad_pawns::evaluate),
];

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

impl Sub<Evaluation> for Evaluation {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Evaluation(self.0 - rhs.0)
    }
}

impl AddAssign<Evaluation> for Evaluation {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl SubAssign<Evaluation> for Evaluation {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
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

impl Mul<f32> for Evaluation {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Evaluation((self.0 as f32 * rhs) as i32)
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

struct StateVariation<'a> {
    state: &'a State,
    end_game_weight: f32,
    piece_counts: ArrayMap<PieceIndex, u8>,
    color_counts: ArrayMap<Color, u8>,
}

impl Deref for StateVariation<'_> {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<'a> From<&'a State> for StateVariation<'a> {
    fn from(state: &'a State) -> Self {
        let (piece_counts, color_counts) = {
            let mut piece_counts = ArrayMap::default();
            let mut color_counts = ArrayMap::default();
            for color in Color::ALL {
                for piece in Piece::ALL {
                    let piece_index = PieceIndex::new(*color, *piece);
                    let count = state.board().piece_occupancy(piece_index).count_ones() as u8;
                    color_counts[*color] += count;
                    piece_counts[piece_index] = count;
                }
            }

            (piece_counts, color_counts)
        };

        let end_game_weight = {
            let count_pieces = |piece: Piece| {
                (piece_counts[PieceIndex::new(Color::White, piece)]
                    + piece_counts[PieceIndex::new(Color::Black, piece)]) as f32
            };

            let w1 = 3.0;
            let v1 = count_pieces(Piece::Pawn) / 16.0;

            let w2 = 1.0;
            let v2 = count_pieces(Piece::Queen) / 2.0;

            let w3 = 1.0;
            let v3 = (state.board().occupancy().count_ones() as f32) / 32.0;

            1.0 - (w1 * v1 + w2 * v2 + w3 * v3) / (w1 + w2 + w3)
        };

        Self {
            state,
            piece_counts,
            color_counts,
            end_game_weight,
        }
    }
}

pub struct Evaluator {
    fns: &'static [(f32, EvaluationFunction)],
}

impl Default for Evaluator {
    fn default() -> Self {
        Self { fns: &EVALUATORS }
    }
}

impl Evaluator {
    #[cfg(test)]
    fn just(fns: &'static [(f32, EvaluationFunction)]) -> Self {
        Self { fns }
    }

    pub fn estimate(&self, state: &State, mv: &Move) -> Evaluation {
        let mut eval = Evaluation::EVEN;

        // Putting a piece where it can be attacked by a pawn is probably bad.
        let pawn_attacks = state.board().colored_pawn_attacks(!mv.color());
        if (pawn_attacks & BitBoard::just(mv.destination())).any() {
            eval -= Evaluation::ONE_PAWN * PIECE_PAWN_WORTHS[mv.piece()];
        }

        // Capturing a piece more valuable is probably good
        if let Some(captured_piece) = mv.capture() {
            // We should check even bad captures, so we make sure this difference is positive.
            eval += Evaluation::ONE_PAWN * PIECE_PAWN_WORTHS[captured_piece] * 10;
            eval -= Evaluation::ONE_PAWN * PIECE_PAWN_WORTHS[mv.piece()];
        }

        // Castling is probably good
        if mv.castle_side().is_some() {
            eval += Evaluation::ONE_PAWN * 2.0;
        }

        // Double pawn push is maybe a little better than a single pawn push
        if mv.is_double_pawn() {
            eval += Evaluation::ONE_PAWN * 0.2;
        }

        // A promotion is probably good
        if let Some(promotion) = mv.promotion() {
            eval += Evaluation::ONE_PAWN * PIECE_PAWN_WORTHS[promotion] * 2.0;
        }

        // Moving the piece from a bad square to a good square is probably good
        if mv.promotion().is_none() {
            let origin_square_value = evaluate_piece_squares::evaluate_piece_square(
                mv.piece(),
                mv.origin(),
                &mv.color(),
                0.5,
            );

            let destination_square_value = evaluate_piece_squares::evaluate_piece_square(
                mv.piece(),
                mv.destination(),
                &mv.color(),
                0.5,
            );

            eval += (destination_square_value - origin_square_value) * 2;
        }

        eval
    }

    pub fn evaluate(&self, state: &State, perspective: Color) -> Evaluation {
        let v = StateVariation::from(state);

        let king_has_move = {
            let king = state
                .board()
                .piece_occupancy(PieceIndex::new(state.turn_to_move(), Piece::King));
            let king_square = king.first_square().unwrap();
            let spaces_around_king = AttackGenerator::compute_king_attacks(king_square);
            let valid_king_squares = spaces_around_king
                & !state.board().occupancy()
                & !state.board().colored_attacks(!state.turn_to_move());

            valid_king_squares.any()
        };

        // If the king can move, we're definitely not in checkmate or stalemate, so we can
        // skip the expensive check for checkmate or stalemate through move generation
        if !king_has_move {
            let legal_moves = MoveGenerator::compute_legal_moves(state);
            if legal_moves.is_empty() && state.is_check() {
                return if state.turn_to_move() == perspective {
                    Evaluation::NEG_INF
                } else {
                    Evaluation::POS_INF
                };
            } else if legal_moves.is_empty() {
                return Evaluation::EVEN;
            }
        }

        let mut eval = Evaluation::EVEN;
        let mut stop = false;

        for (w, f) in self.fns {
            let e = {
                let mut e1 = Evaluation::EVEN;
                f(&v, &perspective, &mut e1, &mut stop);

                let mut e2 = Evaluation::EVEN;
                f(&v, &!perspective, &mut e2, &mut stop);

                e1 - e2
            };

            eval += e * (*w);

            if stop {
                break;
            }
        }

        eval
    }

    pub fn sum_material(&self, state: &State, perspective: &Color) -> Evaluation {
        let mut eval = Evaluation::EVEN;
        for color in Color::ALL {
            let multiplier = if *color == *perspective { 1 } else { -1 };
            for piece in Piece::ALL {
                let piece_index = PieceIndex::new(*color, *piece);
                let piece_occupancy = state.board().piece_occupancy(piece_index);
                let piece_count = piece_occupancy.count_ones() as u8;
                eval += Evaluation::ONE_PAWN
                    * PIECE_PAWN_WORTHS[*piece]
                    * piece_count as i32
                    * multiplier;
            }
        }

        eval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weechess_core::{
        notation::{try_from_notation, Fen},
        State,
    };

    #[test]
    fn test_empty() {
        let evaluator = Evaluator::just(&[]);
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
    fn test_default_even_eval() {
        let evaluator = Evaluator::default();
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
    fn test_normalized_end_game_weight() {
        let game_state = State::default();
        let v = StateVariation::from(&game_state);
        assert_eq!(v.end_game_weight, 0.0);

        let game_state = try_from_notation::<_, Fen>("8/5k2/8/8/2R5/2K5/8/8 w - - 0 1").unwrap();
        let v = StateVariation::from(&game_state);
        assert!(v.end_game_weight > 0.95, "weight={}", v.end_game_weight);
    }

    #[test]
    fn test_clearly_winning() {
        let game_state =
            try_from_notation::<_, Fen>("4k3/8/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 1").unwrap();

        let e1 = Evaluator::default().evaluate(&game_state, Color::White);
        let e2 = Evaluator::default().evaluate(&game_state, Color::Black);
        assert!(e1 > e2);
    }
}
