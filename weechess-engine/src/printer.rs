use std::{borrow::Cow, fmt::Display};

use weechess_core::{
    notation::{into_notation, Fen},
    {utils::ArrayMap, Color, File, Piece, PieceIndex, Rank, Square, State},
};

const BOARD_TEMPLATE_ROWS: &'static [&'static str] = &[
    "   ╭───┬───┬───┬───┬───┬───┬───┬───╮",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ├───┼───┼───┼───┼───┼───┼───┼───┤",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ├───┼───┼───┼───┼───┼───┼───┼───┤",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ├───┼───┼───┼───┼───┼───┼───┼───┤",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ├───┼───┼───┼───┼───┼───┼───┼───┤",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ├───┼───┼───┼───┼───┼───┼───┼───┤",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ├───┼───┼───┼───┼───┼───┼───┼───┤",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ├───┼───┼───┼───┼───┼───┼───┼───┤",
    "r  │ . │ . │ . │ . │ . │ . │ . │ . │",
    "   ╰───┴───┴───┴───┴───┴───┴───┴───╯",
    "                                    ",
    "     f   f   f   f   f   f   f   f  ",
];

pub struct GamePrinter<'a> {
    pub game: Cow<'a, State>,
}

impl<'a> GamePrinter<'a> {
    pub fn new(game: &'a State) -> Self {
        Self {
            game: Cow::Borrowed(game),
        }
    }
}

impl<'a, T> From<T> for GamePrinter<'a>
where
    T: Into<Cow<'a, State>>,
{
    fn from(value: T) -> Self {
        Self { game: value.into() }
    }
}

impl Display for GamePrinter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pieces: ArrayMap<Square, PieceIndex> = self.game.board().into();

        write!(f, "\n")?;
        write!(f, " {}\n\n", into_notation::<_, Fen>(&self.game))?;

        let mut square_index = 0;
        let mut rank_index = 0;
        let mut file_index = 0;
        for line in BOARD_TEMPLATE_ROWS.iter() {
            write!(f, " ")?;

            for c in line.chars() {
                let c = match c {
                    '.' => {
                        let rank = Rank::from_index(square_index / 8).unwrap();
                        let file = File::from_index(square_index % 8).unwrap();
                        let piece = pieces[Square::from((rank.opposing_rank(), file))];
                        square_index += 1;
                        match piece.piece_and_color() {
                            (Piece::Pawn, Color::White) => '♙',
                            (Piece::Pawn, Color::Black) => '♟',
                            (Piece::Knight, Color::White) => '♘',
                            (Piece::Knight, Color::Black) => '♞',
                            (Piece::Bishop, Color::White) => '♗',
                            (Piece::Bishop, Color::Black) => '♝',
                            (Piece::Rook, Color::White) => '♖',
                            (Piece::Rook, Color::Black) => '♜',
                            (Piece::Queen, Color::White) => '♕',
                            (Piece::Queen, Color::Black) => '♛',
                            (Piece::King, Color::White) => '♔',
                            (Piece::King, Color::Black) => '♚',
                            _ => ' ',
                        }
                    }
                    'r' => {
                        let _r = rank_index;
                        rank_index += 1;
                        match _r {
                            0 => '8',
                            1 => '7',
                            2 => '6',
                            3 => '5',
                            4 => '4',
                            5 => '3',
                            6 => '2',
                            7 => '1',
                            _ => ' ',
                        }
                    }
                    'f' => {
                        let _f = file_index;
                        file_index += 1;
                        match _f {
                            0 => 'a',
                            1 => 'b',
                            2 => 'c',
                            3 => 'd',
                            4 => 'e',
                            5 => 'f',
                            6 => 'g',
                            7 => 'h',
                            _ => ' ',
                        }
                    }
                    _ => c,
                };

                write!(f, "{}", c)?;
            }

            write!(f, "\n")?;
        }

        write!(f, "\n\n")?;
        write!(f, "  Turn to move: {}\n", self.game.turn_to_move())?;
        write!(
            f,
            "  En passant target: {:?}\n",
            self.game.en_passant_target()
        )?;

        write!(
            f,
            "  Castle rights: w {}  |  b {}\n",
            self.game.castle_rights(Color::White),
            self.game.castle_rights(Color::Black)
        )?;

        write!(f, "  Clock: {}\n", self.game.clock())?;

        write!(
            f,
            "\n\nhttps://lichess.org/editor?fen={}&variant=standard&color={}\n\n",
            urlencoding::encode(&into_notation::<_, Fen>(&self.game).to_string()),
            match self.game.turn_to_move() {
                Color::White => "white",
                Color::Black => "black",
            }
        )?;

        Ok(())
    }
}
