use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    hasher,
    notation::{try_from_notation, San},
    Move, MoveGenerator, State,
};

#[derive(Serialize, Deserialize)]
pub struct Book {
    table: BTreeMap<hasher::Hash, Vec<Move>>,
}

impl Book {
    pub fn new() -> Self {
        Self {
            table: BTreeMap::new(),
        }
    }

    pub fn find(&self, hash: hasher::Hash) -> Option<&[Move]> {
        self.table.get(&hash).map(|moves| &moves[..])
    }

    pub fn append(&mut self, hash: hasher::Hash, moves: &[Move]) {
        self.table
            .entry(hash)
            .or_insert_with(Vec::new)
            .extend_from_slice(moves);
    }
}

#[derive(Debug)]
pub enum BookParseError {
    InvalidMove,
    UnknownMove,
}

impl std::fmt::Display for BookParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            BookParseError::InvalidMove => write!(f, "invalid move"),
            BookParseError::UnknownMove => write!(f, "unknown move"),
        }
    }
}

impl std::error::Error for BookParseError {}

pub struct BookParser;

impl BookParser {
    pub fn parse_movetext<'a>(
        movetext: &'a str,
        hasher: &'a hasher::ZobristHasher,
    ) -> impl Iterator<Item = Result<(hasher::Hash, Move), BookParseError>> + 'a {
        movetext
            .split_whitespace()
            .filter(|t| match *t {
                "1/2-1/2" => false,
                "1-0" => false,
                "0-1" => false,
                _ => !t.ends_with('.'),
            })
            .scan(
                State::default(),
                |state, move_str| match try_from_notation::<_, San>(move_str) {
                    Ok(query) => {
                        let move_generator = MoveGenerator;
                        let valid_moves = move_generator.compute_legal_moves(state);
                        let Some(result) = valid_moves.find(&query) else {
                            return Some(Err(BookParseError::UnknownMove));
                        };

                        let entry = (hasher.hash(state), result.0);
                        *state = result.1;

                        Some(Ok(entry))
                    }
                    Err(..) => Some(Err(BookParseError::InvalidMove)),
                },
            )
    }
}
