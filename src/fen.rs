use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use regex::Regex;

use crate::game::{
    self, ArrayMap, Board, CastleRights, Clock, Color, File, Piece, PieceIndex, Rank, Square,
};

const DEFAULT_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const FEN_REGEX: &str = r"^(((?:[rnbqkpRNBQKP1-8]+\/){7})[rnbqkpRNBQKP1-8]+)\s([b|w])\s(-|([K|Q|k|q]{1,4}))\s(-|[a-h][1-8])\s(\d+)\s(\d+)$";

mod token {
    pub const WHITE_PAWN: char = 'P';
    pub const WHITE_KNIGHT: char = 'N';
    pub const WHITE_BISHOP: char = 'B';
    pub const WHITE_ROOK: char = 'R';
    pub const WHITE_QUEEN: char = 'Q';
    pub const WHITE_KING: char = 'K';
    pub const BLACK_PAWN: char = 'p';
    pub const BLACK_KNIGHT: char = 'n';
    pub const BLACK_BISHOP: char = 'b';
    pub const BLACK_ROOK: char = 'r';
    pub const BLACK_QUEEN: char = 'q';
    pub const BLACK_KING: char = 'k';
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidCastleRights,
    InvalidFormat,
    InvalidFullmoveNumber,
    InvalidHalfmoveClock,
    InvalidPiece,
    InvalidSquare,
    InvalidTurnToMove,
}

impl TryFrom<char> for PieceIndex {
    type Error = ParseError;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            token::WHITE_PAWN => Ok(Self::new(Color::White, Piece::Pawn)),
            token::WHITE_KNIGHT => Ok(Self::new(Color::White, Piece::Knight)),
            token::WHITE_BISHOP => Ok(Self::new(Color::White, Piece::Bishop)),
            token::WHITE_ROOK => Ok(Self::new(Color::White, Piece::Rook)),
            token::WHITE_QUEEN => Ok(Self::new(Color::White, Piece::Queen)),
            token::WHITE_KING => Ok(Self::new(Color::White, Piece::King)),
            token::BLACK_PAWN => Ok(Self::new(Color::Black, Piece::Pawn)),
            token::BLACK_KNIGHT => Ok(Self::new(Color::Black, Piece::Knight)),
            token::BLACK_BISHOP => Ok(Self::new(Color::Black, Piece::Bishop)),
            token::BLACK_ROOK => Ok(Self::new(Color::Black, Piece::Rook)),
            token::BLACK_QUEEN => Ok(Self::new(Color::Black, Piece::Queen)),
            token::BLACK_KING => Ok(Self::new(Color::Black, Piece::King)),
            _ => Err(ParseError::InvalidPiece),
        }
    }
}

impl TryFrom<&str> for Square {
    type Error = ParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.len() != 2 {
            return Err(ParseError::InvalidSquare);
        }

        let file = s.chars().nth(0).unwrap();
        let file = File::from_char(file).ok_or(ParseError::InvalidSquare)?;

        let rank = s.chars().nth(1).unwrap();
        let rank = Rank::from_char(rank).ok_or(ParseError::InvalidSquare)?;

        Ok(Self::from((file, rank)))
    }
}

impl TryFrom<&str> for ArrayMap<Color, CastleRights> {
    type Error = ParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut result = Self::filled(CastleRights::NONE);
        for c in s.chars() {
            match c {
                token::BLACK_KING => result[Color::Black].kingside = true,
                token::BLACK_QUEEN => result[Color::Black].queenside = true,
                token::WHITE_KING => result[Color::White].kingside = true,
                token::WHITE_QUEEN => result[Color::White].queenside = true,
                '-' => (),
                _ => return Err(ParseError::InvalidCastleRights),
            }
        }

        Ok(result)
    }
}

impl TryFrom<&str> for Board {
    type Error = ParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut map = Board::empty_map();
        let mut location_index: usize = 0;
        for c in s.chars() {
            match c {
                '1'..='8' => location_index += c.to_digit(10).unwrap() as usize,
                ' ' => break,
                '/' => (),
                _ => {
                    let piece = PieceIndex::try_from(c)?;
                    let square = {
                        let square = Square::try_from(location_index)
                            .map_err(|_| ParseError::InvalidSquare)?;

                        let (rank, file) = square.rank_file();

                        // FEN starts with the 8th rank, but square indexes start with the 1st rank.
                        Square::from((file, rank.opposing_rank()))
                    };

                    map[square] = piece;
                    location_index += 1;
                }
            }
        }

        Ok(Board::from(&map))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fen<'a>(Cow<'a, str>);

impl TryFrom<Fen<'_>> for game::State {
    type Error = ParseError;

    fn try_from(fen: Fen<'_>) -> Result<Self, Self::Error> {
        let re = Regex::new(FEN_REGEX).unwrap();
        let groups = re.captures(&fen.0).ok_or(ParseError::InvalidFormat)?;

        let board = Board::try_from(groups.get(1).unwrap().as_str())?;

        let turn_to_move = match groups.get(3).unwrap().as_str() {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(ParseError::InvalidTurnToMove),
        };

        let castle_rights = match groups.get(4).unwrap().as_str() {
            "-" => ArrayMap::filled(CastleRights::NONE),
            s => ArrayMap::try_from(s)?,
        };

        let en_passant_target = match groups.get(6).unwrap().as_str() {
            "-" => None,
            s => Some(Square::try_from(s)?),
        };

        let clock = Clock {
            halfmove_clock: groups
                .get(7)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| ParseError::InvalidHalfmoveClock)?,

            fullmove_number: groups
                .get(8)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| ParseError::InvalidFullmoveNumber)?,
        };

        Ok(Self {
            board,
            turn_to_move,
            castle_rights,
            en_passant_target,
            clock,
        })
    }
}

impl<'a> From<game::State> for Fen<'a> {
    fn from(_state: game::State) -> Self {
        todo!()
    }
}

impl Display for Fen<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Fen<'_> {
    fn default() -> Self {
        Fen(Cow::Borrowed(DEFAULT_FEN))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fen() {
        let fen = Fen::default();
        let state = game::State::try_from(fen).unwrap();
        let board = state.board;

        for square in Square::ALL {
            if let Some(piece) = board.piece_at(*square) {
                println!("{}: [{}]", square, piece);
            } else {
                println!("{}: [ ]", square);
            }
        }

        assert_eq!(
            board.piece_at(Square::A1),
            Some(PieceIndex::new(Color::White, Piece::Rook))
        );

        assert_eq!(
            board.piece_at(Square::E1),
            Some(PieceIndex::new(Color::White, Piece::King))
        );

        assert_eq!(
            board.piece_at(Square::C7),
            Some(PieceIndex::new(Color::Black, Piece::Pawn))
        );

        assert_eq!(board.piece_at(Square::E4), None,);

        assert_eq!(state.turn_to_move, Color::White);
        assert_eq!(state.castle_rights[Color::White], CastleRights::BOTH);
        assert_eq!(state.en_passant_target, None);
    }

    #[test]
    fn test_round_trip() {
        let fen1 = Fen::default();
        let state = game::State::try_from(fen1.clone()).unwrap();
        let fen2 = Fen::from(state);
        assert_eq!(fen1, fen2);
    }
}
