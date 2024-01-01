use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul, Neg},
};

use weechess_core::{
    utils::ArrayMap, Color, MoveGenerationBuffer, MoveGenerator, Piece, PieceIndex, State,
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

    pub fn estimate(&self, state: &State) -> Evaluation {
        // Dumb estimate: add up piece worths
        let mut evaluation = Evaluation::EVEN;
        for color in Color::ALL {
            let mutiplier = if *color == state.turn_to_move() {
                1
            } else {
                -1
            };

            for piece in Piece::ALL {
                let piece_index = PieceIndex::new(*color, *piece);
                let piece_worth = Evaluation::ONE_PAWN * PIECE_WORTHS[*piece];
                let piece_count = state.board().piece_occupancy(piece_index).count_ones() as i32;
                evaluation += Evaluation::from(piece_worth) * mutiplier * piece_count;
            }
        }

        evaluation
    }

    pub fn evaluate(&self, state: &State) -> Evaluation {
        let move_generator = MoveGenerator;
        let mut move_buffer = MoveGenerationBuffer::new();
        move_generator.compute_legal_moves_into(state, &mut move_buffer);

        if move_buffer.legal_moves.is_empty() && state.is_check() {
            return Evaluation::NEG_INF;
        } else if move_buffer.legal_moves.is_empty() {
            return Evaluation::EVEN;
        }

        self.estimate(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let evaluator = Evaluator::new();
        let game_state = State::default();
        assert_eq!(evaluator.evaluate(&game_state), Evaluation::EVEN);
    }
}
