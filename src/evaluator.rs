use std::{
    fmt::Display,
    ops::{Add, AddAssign, Neg},
};

use crate::game::{self};

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

impl Evaluation {
    const ONE_PAWN: i32 = 100;

    pub const NEG_INF: Evaluation = Evaluation(-100 * Self::ONE_PAWN);
    pub const POS_INF: Evaluation = Evaluation(100 * Self::ONE_PAWN);
    pub const EVEN: Evaluation = Evaluation(0);

    pub fn mate_in(_ply: usize) -> Evaluation {
        Evaluation::POS_INF
    }

    pub fn is_terminal(self) -> bool {
        self <= Self::NEG_INF || self >= Self::POS_INF
    }
}

impl Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self >= Self::EVEN {
            write!(f, "+")?;
        }

        write!(f, "{:.1}", (self.0 as f32) / (Self::ONE_PAWN as f32))
    }
}

pub struct Evaluator;

impl Evaluator {
    pub fn new() -> Self {
        Evaluator
    }

    pub fn estimate(&self, _state: &game::State) -> Evaluation {
        // TODO
        Evaluation::EVEN
    }

    pub fn evaluate(&self, state: &game::State) -> Evaluation {
        let move_generator = game::MoveGenerator;
        let mut move_buffer = game::MoveGenerationBuffer::new();
        move_generator.compute_legal_moves_into(state, &mut move_buffer);

        if move_buffer.legal_moves.is_empty() && state.is_check() {
            return Evaluation::NEG_INF;
        } else if move_buffer.legal_moves.is_empty() {
            return Evaluation::EVEN;
        }

        Evaluation::EVEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let evaluator = Evaluator::new();
        let game_state = game::State::default();
        assert_eq!(evaluator.evaluate(&game_state), Evaluation::EVEN);
    }
}
