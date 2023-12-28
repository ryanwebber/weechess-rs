use std::{
    fmt::Display,
    ops::{Add, AddAssign, Neg},
};

use crate::game::{self};

pub type EvaluationFunction = fn(&game::State) -> Evaluation;

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
    pub const NEG_INF: Evaluation = Evaluation(-10000);
    pub const POS_INF: Evaluation = Evaluation(10000);
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
        let value = self.0;
        if value >= 1000 {
            write!(f, "Winning")
        } else if value <= -1000 {
            write!(f, "Losing")
        } else {
            if value > 0 {
                write!(f, "+")?;
            }

            write!(f, "{}", value)
        }
    }
}

pub struct Evaluator {
    fns: Vec<EvaluationFunction>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            fns: vec![|state| {
                let mut value = 0;
                for color in game::Color::ALL {
                    for piece in game::Piece::ALL {
                        let piece_index = game::PieceIndex::new(*color, *piece);
                        let piece_occupancy = state.board().piece_occupancy(piece_index);
                        for _ in piece_occupancy.iter_ones() {
                            if *color == state.turn_to_move() {
                                value += 1
                            } else {
                                value -= 1
                            }
                        }
                    }
                }

                Evaluation(value)
            }],
        }
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
