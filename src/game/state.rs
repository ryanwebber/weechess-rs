use std::fmt::Display;

use crate::fen;

use super::{ArrayMap, Board, Color, Move, MoveGenerator, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastleRights {
    pub kingside: bool,
    pub queenside: bool,
}

impl CastleRights {
    pub const NONE: Self = Self {
        kingside: false,
        queenside: false,
    };

    pub const BOTH: Self = Self {
        kingside: true,
        queenside: true,
    };

    pub fn both(self) -> bool {
        self == Self::BOTH
    }

    pub fn none(self) -> bool {
        self == Self::NONE
    }
}

impl Default for CastleRights {
    fn default() -> Self {
        Self::BOTH
    }
}

impl Display for CastleRights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.none() {
            write!(f, "-")
        } else {
            if self.kingside {
                write!(f, "K")?
            }

            if self.queenside {
                write!(f, "Q")?
            }

            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock {
    pub halfmove_clock: usize,
    pub fullmove_number: usize,
}

impl Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.fullmove_number, self.halfmove_clock)
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            halfmove_clock: 0,
            fullmove_number: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    board: Board,
    turn_to_move: Color,
    castle_rights: ArrayMap<Color, CastleRights>,
    en_passant_target: Option<Square>,
    clock: Clock,
}

impl State {
    pub fn new(
        board: Board,
        turn_to_move: Color,
        castle_rights: ArrayMap<Color, CastleRights>,
        en_passant_target: Option<Square>,
        clock: Clock,
    ) -> Self {
        Self {
            board,
            turn_to_move,
            castle_rights,
            en_passant_target,
            clock,
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn turn_to_move(&self) -> Color {
        self.turn_to_move
    }

    pub fn castle_rights(&self, color: Color) -> CastleRights {
        self.castle_rights[color]
    }

    pub fn en_passant_target(&self) -> Option<Square> {
        self.en_passant_target
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    pub fn by_performing_move(state: &Self, mv: &Move) -> Result<State, ()> {
        let movegen = MoveGenerator;
        let moves = movegen.compute(state);
        let result = moves.iter().find(|m| m.0 == *mv).ok_or(())?.clone();
        Ok(result.1)
    }
}

impl Default for State {
    fn default() -> Self {
        fen::Fen::default().try_into().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let _ = State::default();
    }
}
