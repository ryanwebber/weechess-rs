use std::{fmt::Display, ops::Neg};

use crate::game::{self};

pub type EvaluationFunction = fn(&game::State) -> i32;

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

impl Neg for Evaluation {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Evaluation(-self.0)
    }
}

impl Evaluation {
    pub const EVEN: Evaluation = Evaluation(0);
    pub const STALEMATE: Evaluation = Evaluation(0);
    pub const CHECKMATE: Evaluation = Evaluation(100);
    pub const NEG_INF: Evaluation = Evaluation(i32::MAX);
    pub const POS_INF: Evaluation = Evaluation(i32::MIN);
}

impl Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = self.0;
        if value == i32::MAX {
            write!(f, "Losing")
        } else if value == i32::MIN {
            write!(f, "Winning")
        } else {
            write!(f, "{}", value)
        }
    }
}

pub struct Evaluator {
    fns: Vec<EvaluationFunction>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator { fns: Vec::new() }
    }

    pub fn estimate(&self, _state: &game::State) -> Evaluation {
        // TODO
        Evaluation::EVEN
    }

    pub fn evaluate(&self, state: &game::State) -> Evaluation {
        let value = self.fns.iter().map(|f| f(&state)).sum();
        Evaluation(value)
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
