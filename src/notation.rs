use std::{borrow::Cow, fmt::Display, marker::PhantomData, ops::Deref};

pub use fen::*;
pub use peg::*;

pub trait NotationFormat<Value>: Sized
where
    Value: Sized + Clone,
{
    fn fmt(value: &Value, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
    fn try_parse(notation: &str) -> Result<Notation<'_, Value, Self>, ()>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Notation<'a, T, F>
where
    T: Sized + Clone,
    F: NotationFormat<T>,
{
    value: Cow<'a, T>,
    _marker: PhantomData<F>,
}

impl<T, F> Deref for Notation<'_, T, F>
where
    T: Sized + Clone,
    F: NotationFormat<T>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, F> Display for Notation<'_, T, F>
where
    T: Sized + Clone,
    F: NotationFormat<T>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        F::fmt(self, f)
    }
}

impl<'a, F, T> From<T> for Notation<'a, T, F>
where
    T: Sized + Clone,
    F: NotationFormat<T>,
{
    fn from(value: T) -> Self {
        Self {
            value: Cow::Owned(value),
            _marker: PhantomData,
        }
    }
}

pub fn try_parse<T, F>(s: &str) -> Result<T, ()>
where
    T: Sized + Clone,
    F: NotationFormat<T>,
{
    F::try_parse(s).map(|n| n.value.into_owned())
}

pub fn as_notation<T, F>(value: &T) -> Notation<'_, T, F>
where
    T: Sized + Clone,
    F: NotationFormat<T>,
{
    Notation {
        value: Cow::Borrowed(value),
        _marker: PhantomData,
    }
}

mod peg {
    use super::*;
    use crate::game::{self, Piece, Side};

    pub struct Peg;

    impl NotationFormat<game::Move> for Peg {
        fn fmt(value: &game::Move, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if value.is_any_castle() {
                write!(f, "O-O")?;
                if value.is_castle(Side::Queen) {
                    write!(f, "-O")?;
                }
            } else {
                let piece = value.piece();
                if piece != Piece::Pawn {
                    write!(f, "{}", piece)?;
                }

                let origin = value.origin();
                if value.is_capture() {
                    if piece == Piece::Pawn {
                        write!(f, "{}", origin.file())?;
                    }
                    write!(f, "x")?;
                }

                write!(f, "{}", value.destination())?;

                if let Some(promotion) = value.promotion() {
                    write!(f, "={}", promotion)?;
                }
            }

            Ok(())
        }

        fn try_parse(_notation: &str) -> Result<Notation<'_, game::Move, Self>, ()> {
            todo!()
        }
    }
}

mod fen {
    use super::*;
    use crate::game::{
        self, ArrayMap, Board, CastleRights, Clock, Color, File, Piece, PieceIndex, Rank, Square,
    };

    use regex::Regex;

    const FEN_REGEX: &str = r"^(((?:[rnbqkpRNBQKP1-8]+\/){7})[rnbqkpRNBQKP1-8]+)\s([b|w])\s(-|([K|Q|k|q]{1,4}))\s(-|[a-h][1-8])\s(\d+)\s(\d+)$";

    pub struct Fen;

    impl Fen {
        pub const DEFAULT: &'static str =
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    }

    impl NotationFormat<game::State> for Fen {
        fn fmt(value: &game::State, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            {
                // Write the board.
                let pieces = ArrayMap::from(value.board());
                for rank in Rank::ALL.iter().rev() {
                    let mut empty_squares: i32 = 0;
                    for file in File::ALL.iter() {
                        let square = Square::from((*file, *rank));
                        if pieces[square].some() {
                            if empty_squares > 0 {
                                write!(f, "{}", empty_squares)?;
                                empty_squares = 0;
                            }

                            write!(f, "{}", pieces[square])?;
                        } else {
                            empty_squares += 1;
                        }
                    }

                    if empty_squares > 0 {
                        write!(f, "{}", empty_squares)?;
                    }

                    if *rank != Rank::ONE {
                        write!(f, "/")?;
                    }
                }
            }

            write!(f, " ")?;

            {
                // Write the turn to move.
                let turn_to_move = value.turn_to_move();
                match turn_to_move {
                    Color::White => write!(f, "w")?,
                    Color::Black => write!(f, "b")?,
                }
            }

            write!(f, " ")?;

            {
                // Write the castle rights.
                if Color::ALL.iter().all(|c| value.castle_rights(*c).none()) {
                    write!(f, "-")?;
                } else {
                    if value.castle_rights(Color::White).kingside {
                        write!(f, "K")?;
                    }
                    if value.castle_rights(Color::White).queenside {
                        write!(f, "Q")?;
                    }
                    if value.castle_rights(Color::Black).kingside {
                        write!(f, "k")?;
                    }
                    if value.castle_rights(Color::Black).queenside {
                        write!(f, "q")?;
                    }
                }
            }

            write!(f, " ")?;

            {
                // Write the en passant target.
                let en_passant_target = value.en_passant_target();
                match en_passant_target {
                    None => write!(f, "-")?,
                    Some(square) => write!(f, "{}", square)?,
                }
            }

            write!(f, " ")?;

            {
                // Write the halfmove clock.
                let halfmove_clock = value.clock().halfmove_clock;
                write!(f, "{}", halfmove_clock)?;
            }

            write!(f, " ")?;

            {
                // Write the fullmove number.
                let fullmove_number = value.clock().fullmove_number;
                write!(f, "{}", fullmove_number)?;
            }

            Ok(())
        }

        fn try_parse(notation: &str) -> Result<Notation<'_, game::State, Self>, ()> {
            let re = Regex::new(FEN_REGEX).unwrap();
            let groups = re.captures(notation).ok_or(())?;

            let board = Board::try_parse(&groups[1])?;

            let turn_to_move = match &groups[3] {
                "w" => Color::White,
                "b" => Color::Black,
                _ => return Err(()),
            };

            let castle_rights = match &groups[4] {
                "-" => ArrayMap::filled(CastleRights::NONE),
                s => ArrayMap::try_parse(s)?,
            };

            let en_passant_target = match &groups[6] {
                "-" => None,
                s => Some(Square::try_parse(s)?),
            };

            let clock = Clock {
                halfmove_clock: groups[7].parse().map_err(|_| ())?,
                fullmove_number: groups[8].parse().map_err(|_| ())?,
            };

            Ok(
                game::State::new(board, turn_to_move, castle_rights, en_passant_target, clock)
                    .into(),
            )
        }
    }

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

    impl PieceIndex {
        fn try_parse(c: char) -> Result<Self, ()> {
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
                _ => Err(()),
            }
        }
    }

    impl Square {
        fn try_parse(s: &str) -> Result<Self, ()> {
            if s.len() != 2 {
                return Err(());
            }

            let file = s.chars().nth(0).ok_or(())?;
            let file = File::from_char(file).ok_or(())?;

            let rank = s.chars().nth(1).ok_or(())?;
            let rank = Rank::from_char(rank).ok_or(())?;

            Ok(Self::from((file, rank)))
        }
    }

    impl ArrayMap<Color, CastleRights> {
        fn try_parse(s: &str) -> Result<Self, ()> {
            let mut result = Self::filled(CastleRights::NONE);
            for c in s.chars() {
                match c {
                    token::BLACK_KING => result[Color::Black].kingside = true,
                    token::BLACK_QUEEN => result[Color::Black].queenside = true,
                    token::WHITE_KING => result[Color::White].kingside = true,
                    token::WHITE_QUEEN => result[Color::White].queenside = true,
                    '-' => (),
                    _ => return Err(()),
                }
            }

            Ok(result)
        }
    }

    impl Board {
        fn try_parse(s: &str) -> Result<Self, ()> {
            let mut map = Board::empty_map();
            let mut location_index: u8 = 0;
            for c in s.chars() {
                match c {
                    '1'..='8' => location_index += c.to_digit(10).ok_or(())? as u8,
                    ' ' => break,
                    '/' => (),
                    _ => {
                        let piece = PieceIndex::try_parse(c)?;
                        let square = {
                            let square = Square::try_from(location_index).map_err(|_| ())?;

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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_default_fen() {
            let fen = Fen::DEFAULT;
            let state = try_parse::<game::State, Fen>(fen).unwrap();
            let board = state.board();

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

            assert_eq!(state.turn_to_move(), Color::White);
            assert_eq!(state.castle_rights(Color::White), CastleRights::BOTH);
            assert_eq!(state.en_passant_target(), None);
        }

        #[test]
        fn test_round_trip() {
            let fen1 = Fen::DEFAULT;
            let state = Fen::try_parse(fen1).unwrap();
            let fen2 = state.to_string();
            assert_eq!(fen1, fen2);
        }
    }
}
