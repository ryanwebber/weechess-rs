use std::ops::Neg;

use crate::game::{self};

pub type EvaluationFunction = dyn Fn(&game::State) -> f32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluation(f32);

impl From<f32> for Evaluation {
    fn from(value: f32) -> Self {
        Evaluation(value)
    }
}

impl From<Evaluation> for f32 {
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
    pub const EVEN: Evaluation = Evaluation(0.0);
    pub const CHECKMATE: Evaluation = Evaluation(100.0);
}

pub struct Evaluator {
    fns: Vec<Box<EvaluationFunction>>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator { fns: Vec::new() }
    }

    pub fn evaluate(&self, state: game::State) -> Evaluation {
        let value = self.fns.iter().map(|f| f(&state)).sum();
        Evaluation(value)
    }
}

pub struct Builder {
    fns: Vec<Box<EvaluationFunction>>,
}

impl Builder {
    pub fn new() -> Self {
        Builder { fns: Vec::new() }
    }

    pub fn with<F>(mut self, f: F) -> Self
    where
        F: Fn(&game::State) -> f32 + 'static,
    {
        self.fns.push(Box::new(f));
        self
    }

    pub fn build(self) -> Evaluator {
        Evaluator { fns: self.fns }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let evaluator = Evaluator::new();
        let game_state = game::State::default();
        assert_eq!(evaluator.evaluate(game_state), Evaluation::EVEN);
    }
}
