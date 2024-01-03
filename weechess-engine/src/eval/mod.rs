use std::{
    borrow::Cow,
    fmt::Display,
    ops::{Add, AddAssign, Deref, Mul, Neg},
};

use weechess_core::{
    utils::ArrayMap, Color, MoveGenerationBuffer, MoveGenerator, MoveResult, Piece, PieceIndex,
    State,
};

mod estimate_piece_worths;
mod evaluate_piece_squares;

type EvaluationFunction =
    fn(v: &StateVariation<'_>, perspective: &Color, eval: &mut Evaluation, stop: &mut bool);

const EVALUATORS: &'static [EvaluationFunction] = &[
    //
    evaluate_piece_squares::evaluate,
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

struct StateVariation<'a> {
    state: &'a State,
    legal_moves: Cow<'a, [MoveResult]>,
    end_game_weight: f32,
    _piece_counts: ArrayMap<PieceIndex, u8>,
}

impl Deref for StateVariation<'_> {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<'a> From<&'a State> for StateVariation<'a> {
    fn from(state: &'a State) -> Self {
        let mut move_buffer = MoveGenerationBuffer::new();
        MoveGenerator.compute_legal_moves_into(state, &mut move_buffer);

        let piece_counts = {
            let mut counts = ArrayMap::default();
            for color in Color::ALL {
                for piece in Piece::ALL {
                    let piece_index = PieceIndex::new(*color, *piece);
                    counts[piece_index] =
                        state.board().piece_occupancy(piece_index).count_ones() as u8;
                }
            }

            counts
        };

        let end_game_weight = {
            let large_pieces = [
                PieceIndex::new(Color::White, Piece::Rook),
                PieceIndex::new(Color::White, Piece::Queen),
                PieceIndex::new(Color::Black, Piece::Rook),
                PieceIndex::new(Color::Black, Piece::Queen),
            ]
            .iter()
            .map(|p| piece_counts[*p] as u32)
            .sum::<u32>();

            1.0 - (large_pieces as f32 / 6.0).min(1.0)
        };

        Self {
            state,
            _piece_counts: piece_counts,
            end_game_weight,
            legal_moves: Cow::Owned(move_buffer.legal_moves.to_owned()),
        }
    }
}

pub struct Evaluator {
    fns: &'static [EvaluationFunction],
}

impl Default for Evaluator {
    fn default() -> Self {
        Self { fns: &EVALUATORS }
    }
}

impl Evaluator {
    #[cfg(test)]
    fn just(fns: &'static [EvaluationFunction]) -> Self {
        Self { fns }
    }

    pub fn estimate(&self, state: &State, perspective: Color) -> Evaluation {
        estimate_piece_worths::estimate(state, &perspective)
    }

    pub fn evaluate(&self, state: &State, perspective: Color) -> Evaluation {
        let v = StateVariation::from(state);

        // First, check for checkmate or stalemate.
        if v.legal_moves.is_empty() && state.is_check() {
            return if state.turn_to_move() == perspective {
                Evaluation::NEG_INF
            } else {
                Evaluation::POS_INF
            };
        } else if v.legal_moves.is_empty() {
            return Evaluation::EVEN;
        }

        let mut eval = self.estimate(state, perspective);
        let mut stop = false;

        for f in self.fns {
            f(&v, &perspective, &mut eval, &mut stop);
            if stop {
                break;
            }
        }

        eval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weechess_core::State;

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
}
