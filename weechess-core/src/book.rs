use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    hasher,
    notation::{try_from_notation, San},
    Move, MoveGenerator, MoveQuery, State,
};

#[derive(Serialize, Deserialize)]
pub struct Book {
    table: BTreeMap<hasher::Hash, HashSet<Move>>,
}

impl Book {
    pub fn new() -> Self {
        Self {
            table: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&hasher::Hash, &HashSet<Move>)> {
        self.table.iter()
    }

    pub fn find(&self, hash: hasher::Hash) -> Option<&HashSet<Move>> {
        self.table.get(&hash)
    }

    pub fn append(&mut self, hash: hasher::Hash, moves: &[Move]) {
        self.table
            .entry(hash)
            .or_insert_with(HashSet::new)
            .extend(moves);
    }
}

#[derive(Debug)]
pub enum BookParseError {
    InvalidMoveStr(String),
    UnknownMove(String, MoveQuery),
}

impl std::fmt::Display for BookParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BookParseError::InvalidMoveStr(str) => write!(f, "invalid move string: {}", str),
            BookParseError::UnknownMove(str, query) => {
                write!(f, "unknown move: {} for {}", str, query)
            }
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
            .map(|c| {
                if let Some(dot_index) = c.find('.') {
                    &c[dot_index + 1..]
                } else {
                    c
                }
            })
            .scan(
                State::default(),
                |state, move_str| match try_from_notation::<_, San>(move_str) {
                    Ok(query) => {
                        let valid_moves = MoveGenerator::compute_legal_moves(state);
                        let Some(result) = valid_moves.find(&query) else {
                            println!("Unknown move '{}': {}", move_str, query);
                            println!("Game state:\n{}", state.pretty());
                            println!("Available moves:");
                            for m in valid_moves.moves() {
                                println!("  {}", m.0);
                            }

                            return Some(Err(BookParseError::UnknownMove(
                                move_str.to_string(),
                                query,
                            )));
                        };

                        let entry = (hasher.hash(state), result.0);
                        *state = result.1;

                        Some(Ok(entry))
                    }
                    Err(..) => Some(Err(BookParseError::InvalidMoveStr(move_str.to_string()))),
                },
            )
    }
}
