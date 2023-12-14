use crate::fen;

use super::{ArrayMap, Board, Color, Square};

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
}

impl Default for CastleRights {
    fn default() -> Self {
        Self::BOTH
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock {
    pub halfmove_clock: usize,
    pub fullmove_number: usize,
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
    pub board: Board,
    pub turn_to_move: Color,
    pub castle_rights: ArrayMap<Color, CastleRights>,
    pub en_passant_target: Option<Square>,
    pub clock: Clock,
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
