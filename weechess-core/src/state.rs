use std::fmt::Display;

use crate::{
    notation::{self, Fen},
    GamePrinter,
};

use super::{
    utils::ArrayMap, Board, Color, File, Move, MoveGenerator, MoveQuery, MoveResult, Piece,
    PieceIndex, Side, Square,
};

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

    pub fn for_side(self, side: Side) -> bool {
        match side {
            Side::King => self.kingside,
            Side::Queen => self.queenside,
        }
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
pub enum MovePerformError {
    AmbiguousMove,
    IllegalEnPassant,
    UnknownMove,
}

impl Display for MovePerformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MovePerformError::AmbiguousMove => write!(f, "Ambiguous move"),
            MovePerformError::IllegalEnPassant => write!(f, "Invalid en passant move"),
            MovePerformError::UnknownMove => write!(f, "Unknown move"),
        }
    }
}

impl std::error::Error for MovePerformError {}

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

    pub fn is_check(&self) -> bool {
        self.board.is_check(self.turn_to_move)
    }

    pub fn pretty<'a>(&'a self) -> impl Display + 'a {
        GamePrinter::new(self)
    }

    pub fn by_performing_move(state: &Self, mv: &Move) -> Result<State, MovePerformError> {
        let board = {
            let mut map = state.board().piece_map().clone();

            let moving_piece = PieceIndex::new(state.turn_to_move, mv.piece());
            let moving_color = state.turn_to_move;
            let opposing_color = moving_color.opposing_color();

            // Update the start and end positions of the moving piece
            map[moving_piece].set(mv.origin(), false);
            map[moving_piece].set(mv.destination(), true);

            if mv.is_en_passant() {
                let en_passant_target = state
                    .en_passant_target
                    .ok_or(MovePerformError::IllegalEnPassant)?;

                let capture = PieceIndex::new(opposing_color, Piece::Pawn);
                let capture_square = en_passant_target
                    .offset(moving_color.backward())
                    .ok_or(MovePerformError::IllegalEnPassant)?;

                map[capture].set(capture_square, false);
            } else if let Some(capture) = mv.capture() {
                let capture = PieceIndex::new(opposing_color, capture);
                map[capture].set(mv.destination(), false);
            }

            if let Some(promotion) = mv.promotion() {
                let promotion = PieceIndex::new(moving_color, promotion);
                map[moving_piece].set(mv.destination(), false);
                map[promotion].set(mv.destination(), true);
            }

            if mv.is_castle(Side::King) {
                let rook_start = Square::from((mv.origin().rank(), File::H));
                let rook_end = Square::from((mv.origin().rank(), File::F));
                let rook = PieceIndex::new(moving_color, Piece::Rook);
                map[rook].set(rook_start, false);
                map[rook].set(rook_end, true);
            } else if mv.is_castle(Side::Queen) {
                let rook_start = Square::from((mv.origin().rank(), File::A));
                let rook_end = Square::from((mv.origin().rank(), File::D));
                let rook = PieceIndex::new(moving_color, Piece::Rook);
                map[rook].set(rook_start, false);
                map[rook].set(rook_end, true);
            }

            Board::new(map)
        };

        let castle_rights = {
            let mut castle_rights = state.castle_rights.clone();

            if mv.piece() == Piece::King {
                castle_rights[state.turn_to_move] = CastleRights::NONE;
            }

            castle_rights[Color::Black].kingside &= board
                .piece_occupancy(PieceIndex::new(Color::Black, Piece::Rook))
                .test(Square::H8);

            castle_rights[Color::Black].queenside &= board
                .piece_occupancy(PieceIndex::new(Color::Black, Piece::Rook))
                .test(Square::A8);

            castle_rights[Color::White].kingside &= board
                .piece_occupancy(PieceIndex::new(Color::White, Piece::Rook))
                .test(Square::H1);

            castle_rights[Color::White].queenside &= board
                .piece_occupancy(PieceIndex::new(Color::White, Piece::Rook))
                .test(Square::A1);

            castle_rights
        };

        Ok(State {
            board,
            castle_rights,
            turn_to_move: state.turn_to_move().opposing_color(),
            en_passant_target: if mv.is_double_pawn() {
                mv.destination().offset(state.turn_to_move.backward())
            } else {
                None
            },
            clock: Clock {
                halfmove_clock: if mv.is_capture() || mv.piece() == Piece::Pawn {
                    0
                } else {
                    state.clock.halfmove_clock + 1
                },
                fullmove_number: if state.turn_to_move == Color::Black {
                    state.clock.fullmove_number + 1
                } else {
                    state.clock.fullmove_number
                },
            },
        })
    }

    pub fn by_performing_moves(
        state: &Self,
        moves: &[MoveQuery],
    ) -> Result<State, MovePerformError> {
        let mut state = state.clone();
        for mv in moves {
            let move_set = MoveGenerator::compute_legal_moves(&state);
            let valid_moves: Vec<&MoveResult> = move_set.filter(*mv).collect();
            match valid_moves[..] {
                [mv] => {
                    state = Self::by_performing_move(&state, &mv.0)?;
                }
                [] => {
                    return Err(MovePerformError::UnknownMove);
                }
                _ => {
                    return Err(MovePerformError::AmbiguousMove);
                }
            }
        }

        Ok(state)
    }
}

impl Default for State {
    fn default() -> Self {
        let str = notation::Fen::DEFAULT;
        notation::try_from_notation::<_, Fen>(str).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::notation::{into_notation, San};

    use super::*;

    #[test]
    fn test_default_state() {
        let _ = State::default();
    }

    #[test]
    fn test_apply_en_passant_move() {
        let state = notation::try_from_notation::<_, Fen>(
            "r1bq2k1/3nb1pp/p2p2r1/Pp1P1p2/1BN1p2P/6P1/1PPQ1P2/R3KB1R w KQ b6 0 18",
        )
        .unwrap();

        let move_query = notation::try_from_notation::<_, San>("axb6").unwrap();

        let new_state = State::by_performing_moves(&state, &[move_query]).unwrap();
        assert_eq!(
            into_notation::<_, Fen>(&new_state).to_string(),
            "r1bq2k1/3nb1pp/pP1p2r1/3P1p2/1BN1p2P/6P1/1PPQ1P2/R3KB1R b KQ - 0 18",
            "{}",
            new_state.pretty()
        );
    }
}
