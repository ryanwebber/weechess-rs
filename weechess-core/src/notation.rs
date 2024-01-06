use std::{borrow::Cow, fmt::Display, marker::PhantomData, ops::Deref};

pub use fen::*;
pub use peg::*;
pub use san::*;

pub trait IntoNotation<Value>
where
    Value: ?Sized,
{
    fn into_notation(value: &Value, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

pub trait TryFromNotation<Value> {
    type Error;
    fn try_from_notation(notation: &str) -> Result<Value, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Notation<'a, T, F>
where
    T: Sized + Clone,
{
    value: Cow<'a, T>,
    _marker: PhantomData<F>,
}

impl<T, F> Deref for Notation<'_, T, F>
where
    T: Sized + Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, F> Display for Notation<'_, T, F>
where
    T: Sized + Clone,
    F: IntoNotation<T>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        F::into_notation(self, f)
    }
}

impl<'a, F, T> From<T> for Notation<'a, T, F>
where
    T: Sized + Clone,
{
    fn from(value: T) -> Self {
        Self {
            value: Cow::Owned(value),
            _marker: PhantomData,
        }
    }
}

pub fn try_from_notation<T, F>(s: &str) -> Result<T, ()>
where
    T: Sized + Clone,
    F: TryFromNotation<T>,
{
    F::try_from_notation(s).map_err(|_| ())
}

pub fn into_notation<T, F>(value: &T) -> Notation<'_, T, F>
where
    T: Sized + Clone,
    F: IntoNotation<T>,
{
    Notation {
        value: Cow::Borrowed(value),
        _marker: PhantomData,
    }
}

mod peg {
    use super::*;
    use crate::{Move, Piece, Side};

    pub struct Peg;

    impl IntoNotation<Move> for Peg {
        fn into_notation(value: &Move, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    }
}

mod fen {
    use super::*;
    use crate::{
        utils::ArrayMap, Board, CastleRights, Clock, Color, File, Piece, PieceIndex, Rank, Square,
        State,
    };

    use regex::Regex;

    const FEN_REGEX: &str = r"^(((?:[rnbqkpRNBQKP1-8]+\/){7})[rnbqkpRNBQKP1-8]+)\s([b|w])\s(-|([K|Q|k|q]{1,4}))\s(-|[a-h][1-8])\s(\d+)\s(\d+)$";

    pub struct Fen;

    impl Fen {
        pub const DEFAULT: &'static str =
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    }

    impl IntoNotation<State> for Fen {
        fn into_notation(value: &State, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    }

    impl TryFromNotation<State> for Fen {
        type Error = ();

        fn try_from_notation(notation: &str) -> Result<State, Self::Error> {
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
                s => Some(Square::try_from(s)?),
            };

            let clock = Clock {
                halfmove_clock: groups[7].parse().map_err(|_| ())?,
                fullmove_number: groups[8].parse().map_err(|_| ())?,
            };

            Ok(State::new(
                board,
                turn_to_move,
                castle_rights,
                en_passant_target,
                clock,
            ))
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
            let state = try_from_notation::<State, Fen>(fen).unwrap();
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
            let state = try_from_notation::<_, Fen>(fen1).unwrap();
            let fen2 = into_notation::<_, Fen>(&state).to_string();
            assert_eq!(fen1, fen2);
        }
    }
}

mod san {
    use crate::{File, MoveQuery, Piece, Rank, Side};

    use super::TryFromNotation;

    pub struct San;

    impl TryFromNotation<MoveQuery> for San {
        type Error = ();

        fn try_from_notation(notation: &str) -> Result<MoveQuery, Self::Error> {
            // First, check for simple castles
            if notation.starts_with("O-O-O") {
                return Ok(MoveQuery::by_castling(Side::Queen));
            } else if notation.starts_with("O-O") {
                return Ok(MoveQuery::by_castling(Side::King));
            }

            let mut query = MoveQuery::new();
            let mut iter = notation.chars().rev().peekable();

            // Skip the check/checkmate indicator
            if iter.peek() == Some(&'#') || iter.peek() == Some(&'+') {
                iter.next();
            }

            // Promotion
            if let Some(c) = iter.peek().copied() {
                if c.is_ascii_uppercase() {
                    iter.next();

                    match c {
                        'Q' => query.set_promotion(Piece::Queen),
                        'R' => query.set_promotion(Piece::Rook),
                        'B' => query.set_promotion(Piece::Bishop),
                        'N' => query.set_promotion(Piece::Knight),
                        _ => return Err(()),
                    }

                    if iter.peek() == Some(&'=') {
                        iter.next();
                    }
                }
            }

            // Destination rank
            if let Some(r) = iter.peek().copied() {
                if r.is_ascii_digit() {
                    iter.next();

                    match r {
                        '1' => query.set_destination_rank(Rank::ONE),
                        '2' => query.set_destination_rank(Rank::TWO),
                        '3' => query.set_destination_rank(Rank::THREE),
                        '4' => query.set_destination_rank(Rank::FOUR),
                        '5' => query.set_destination_rank(Rank::FIVE),
                        '6' => query.set_destination_rank(Rank::SIX),
                        '7' => query.set_destination_rank(Rank::SEVEN),
                        '8' => query.set_destination_rank(Rank::EIGHT),
                        _ => return Err(()),
                    }
                }
            }

            // Destination file
            if let Some(f) = iter.peek().copied() {
                if f.is_ascii_lowercase() {
                    iter.next();

                    match f {
                        'a' => query.set_destination_file(File::A),
                        'b' => query.set_destination_file(File::B),
                        'c' => query.set_destination_file(File::C),
                        'd' => query.set_destination_file(File::D),
                        'e' => query.set_destination_file(File::E),
                        'f' => query.set_destination_file(File::F),
                        'g' => query.set_destination_file(File::G),
                        'h' => query.set_destination_file(File::H),
                        _ => return Err(()),
                    }
                }
            }

            // Captures
            if iter.peek() == Some(&'x') {
                iter.next();
                query.set_is_capture(true);
            }

            // Origin rank
            if let Some(r) = iter.peek().copied() {
                if r.is_ascii_digit() {
                    iter.next();

                    match r {
                        '1' => query.set_origin_rank(Rank::ONE),
                        '2' => query.set_origin_rank(Rank::TWO),
                        '3' => query.set_origin_rank(Rank::THREE),
                        '4' => query.set_origin_rank(Rank::FOUR),
                        '5' => query.set_origin_rank(Rank::FIVE),
                        '6' => query.set_origin_rank(Rank::SIX),
                        '7' => query.set_origin_rank(Rank::SEVEN),
                        '8' => query.set_origin_rank(Rank::EIGHT),
                        _ => return Err(()),
                    }
                }
            }

            // Origin file
            if let Some(f) = iter.peek().copied() {
                if f.is_ascii_lowercase() {
                    iter.next();

                    match f {
                        'a' => query.set_origin_file(File::A),
                        'b' => query.set_origin_file(File::B),
                        'c' => query.set_origin_file(File::C),
                        'd' => query.set_origin_file(File::D),
                        'e' => query.set_origin_file(File::E),
                        'f' => query.set_origin_file(File::F),
                        'g' => query.set_origin_file(File::G),
                        'h' => query.set_origin_file(File::H),
                        _ => return Err(()),
                    }
                }
            }

            // Origin piece type
            if let Some(p) = iter.peek().copied() {
                if p.is_ascii_uppercase() {
                    iter.next();

                    match p {
                        'K' => query.set_piece(Piece::King),
                        'Q' => query.set_piece(Piece::Queen),
                        'R' => query.set_piece(Piece::Rook),
                        'B' => query.set_piece(Piece::Bishop),
                        'N' => query.set_piece(Piece::Knight),
                        'P' => query.set_piece(Piece::Pawn),
                        _ => return Err(()),
                    }
                }
            }

            // At this point we should have no more characters left
            if iter.peek().is_some() {
                return Err(());
            }

            // If we have no piece type, it's a pawn move
            if query.piece.is_none() {
                query.set_piece(Piece::Pawn);
            }

            Ok(query)
        }
    }
}

pub mod lan {
    use crate::Move;

    use super::{into_notation, IntoNotation};

    pub struct Lan;

    impl IntoNotation<Move> for Lan {
        fn into_notation(value: &Move, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}{}", value.origin(), value.destination())?;
            if let Some(promotion) = value.promotion() {
                write!(f, "{}", Into::<char>::into(promotion).to_ascii_lowercase())?;
            }

            Ok(())
        }
    }

    impl IntoNotation<&[Move]> for Lan {
        fn into_notation(value: &&[Move], f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for (i, mv) in value.iter().enumerate() {
                write!(f, "{}", into_notation::<_, Lan>(mv))?;
                if i < value.len() - 1 {
                    write!(f, " ")?;
                }
            }

            Ok(())
        }
    }
}
